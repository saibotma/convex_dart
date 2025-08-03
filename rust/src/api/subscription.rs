use std::collections::BTreeMap;
use convex::{Value, QuerySubscription, FunctionResult};
use flutter_rust_bridge::frb;
use tokio::sync::Mutex;
use std::sync::Arc;
use futures::StreamExt;

use super::convex_client::{ConvexValue, ConvexError, ConvexClientWrapper};

// Helper function to convert Convex Value to proper JSON string
fn convex_value_to_json(value: Value) -> String {
    // Convert to serde_json::Value first, then serialize to string
    match convex_value_to_serde_json(value) {
        Ok(json_val) => serde_json::to_string(&json_val).unwrap_or_else(|_| "null".to_string()),
        Err(_) => "null".to_string(),
    }
}

// Helper function to convert Convex Value to serde_json::Value
fn convex_value_to_serde_json(value: Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Use the existing debug format to get string representation, then try to parse as JSON
    let debug_str = format!("{:?}", value);
    
    // Handle known patterns
    if debug_str == "Null" {
        return Ok(serde_json::Value::Null);
    }
    
    if let Some(str_val) = debug_str.strip_prefix("String(\"").and_then(|s| s.strip_suffix("\")")) {
        return Ok(serde_json::Value::String(str_val.to_string()));
    }
    
    if let Some(int_str) = debug_str.strip_prefix("Int64(").and_then(|s| s.strip_suffix(")")) {
        if let Ok(int_val) = int_str.parse::<i64>() {
            return Ok(serde_json::Value::Number(serde_json::Number::from(int_val)));
        }
    }
    
    if let Some(float_str) = debug_str.strip_prefix("Float64(").and_then(|s| s.strip_suffix(")")) {
        if let Ok(float_val) = float_str.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(float_val) {
                return Ok(serde_json::Value::Number(num));
            }
        }
    }
    
    // For Array and Object, we need to parse the debug string more carefully
    if debug_str.starts_with("Array([") && debug_str.ends_with("])") {
        // This is a complex case - for now, try to parse the inner content
        let inner = &debug_str[7..debug_str.len()-2]; // Remove "Array([" and "])"
        
        // If it's empty array
        if inner.is_empty() {
            return Ok(serde_json::Value::Array(vec![]));
        }
        
        // For now, let's create a simple array representation
        // This is a simplified approach - a full parser would be more complex
        return Ok(serde_json::Value::Array(vec![serde_json::Value::String(inner.to_string())]));
    }
    
    if debug_str.starts_with("Object({") && debug_str.ends_with("})") {
        // Similar to array, this is complex - for now create a simple object
        let inner = &debug_str[8..debug_str.len()-2]; // Remove "Object({" and "})"
        
        if inner.is_empty() {
            return Ok(serde_json::Value::Object(serde_json::Map::new()));
        }
        
        // For now, create a simple object with the debug string as a value
        let mut obj = serde_json::Map::new();
        obj.insert("debug".to_string(), serde_json::Value::String(inner.to_string()));
        return Ok(serde_json::Value::Object(obj));
    }
    
    // Fallback: return the debug string as a string value
    Ok(serde_json::Value::String(debug_str))
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
                        let result_string = convex_value_to_json(val);
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