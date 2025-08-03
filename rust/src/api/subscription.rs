use std::collections::BTreeMap;
use convex::{Value, QuerySubscription, FunctionResult};
use flutter_rust_bridge::frb;
use tokio::sync::Mutex;
use std::sync::Arc;
use futures::StreamExt;

use super::convex_client::{ConvexValue, ConvexError, ConvexClientWrapper};

// Helper function to convert convex::Value to our ConvexValue enum
fn convert_value(value: Value) -> ConvexValue {
    match value {
        Value::Null => ConvexValue::Null,
        Value::Boolean(b) => ConvexValue::String(b.to_string()), // Store boolean as string since we don't have Bool variant
        Value::String(s) => ConvexValue::String(s),
        Value::Int64(i) => ConvexValue::Int64(i),
        Value::Float64(f) => ConvexValue::Float64(f),
        Value::Array(arr) => {
            let converted_arr: Vec<ConvexValue> = arr.into_iter().map(convert_value).collect();
            ConvexValue::Array(converted_arr)
        },
        Value::Object(obj) => {
            let mut converted_obj = std::collections::HashMap::new();
            for (key, val) in obj {
                converted_obj.insert(key, convert_value(val));
            }
            ConvexValue::Object(converted_obj)
        },
        Value::Bytes(bytes) => ConvexValue::Bytes(bytes),
    }
}

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
                        let result_value = convert_value(val);
                        Some(result_value)
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
            // Convert our ConvexValue back to a String for Convex API
            let string_val = match value {
                ConvexValue::String(s) => s,
                ConvexValue::Int64(i) => i.to_string(),
                ConvexValue::Float64(f) => f.to_string(),
                ConvexValue::Null => "null".to_string(),
                _ => value.to_json_string(), // Fallback to JSON string
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