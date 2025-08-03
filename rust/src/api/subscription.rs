use std::collections::BTreeMap;
use convex::{Value, QuerySubscription, FunctionResult};
use flutter_rust_bridge::frb;
use tokio::sync::Mutex;
use std::sync::Arc;
use futures::StreamExt;

use super::convex_client::{ConvexValue, ConvexError, ConvexClientWrapper};

pub struct ConvexSubscription {
    subscription: Arc<Mutex<Option<QuerySubscription>>>,
}

impl ConvexSubscription {
    async fn create_internal(
        subscription: QuerySubscription,
    ) -> Result<Self, ConvexError> {
        Ok(Self {
            subscription: Arc::new(Mutex::new(Some(subscription))),
        })
    }

    pub async fn next(&self) -> Option<ConvexValue> {
        let mut guard = self.subscription.lock().await;
        if let Some(subscription) = guard.as_mut() {
            match subscription.next().await {
                Some(result) => match result {
                    FunctionResult::Value(val) => {
                        let result_string = match val {
                            Value::String(s) => format!("\"{}\"", s),
                            _ => format!("{:?}", val),
                        };
                        Some(ConvexValue { inner: result_string })
                    }
                    FunctionResult::ErrorMessage(_) => None,
                    FunctionResult::ConvexError(_) => None,
                },
                None => None,
            }
        } else {
            None
        }
    }

    #[frb(sync)]
    pub fn close(&self) {
        // Note: The actual subscription will be dropped when the Rust object is dropped
        // For now, we just clear the subscription reference
        let subscription = self.subscription.clone();
        tokio::spawn(async move {
            let mut guard = subscription.lock().await;
            *guard = None;
        });
    }
}

// Extended ConvexClientWrapper with subscription support
impl ConvexClientWrapper {
    pub async fn subscribe(
        &self,
        function_name: String,
        args: Vec<(String, ConvexValue)>,
    ) -> Result<ConvexSubscription, ConvexError> {
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or_else(|| ConvexError {
            message: "Client not connected".to_string(),
        })?;

        let mut btree_args = BTreeMap::new();
        for (key, value) in args {
            // For now, we'll just work with strings and let Convex handle the conversion
            // This is a simplified approach for the bridge
            let string_val = if value.inner.starts_with('"') && value.inner.ends_with('"') {
                // It's a JSON string, extract the inner value
                value.inner[1..value.inner.len()-1].to_string()
            } else {
                value.inner
            };
            btree_args.insert(key, Value::String(string_val));
        }

        let subscription = client
            .subscribe(&function_name, btree_args)
            .await
            .map_err(|e| ConvexError {
                message: format!("Subscription failed: {}", e),
            })?;

        ConvexSubscription::create_internal(subscription).await
    }
}