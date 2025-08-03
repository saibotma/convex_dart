use std::collections::BTreeMap;
use convex::{ConvexClient, Value, FunctionResult};
use flutter_rust_bridge::frb;
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct ConvexClientWrapper {
    pub(crate) client: Arc<Mutex<Option<ConvexClient>>>,
}

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

#[derive(Debug, Clone)]
pub struct ConvexValue {
    pub inner: String, // JSON serialized value
}

#[derive(Debug, Clone)]
pub struct ConvexError {
    pub message: String,
}

impl ConvexClientWrapper {
    #[frb(sync)]
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self, deployment_url: String) -> Result<(), ConvexError> {
        let client = ConvexClient::new(&deployment_url)
            .await
            .map_err(|e| ConvexError {
                message: format!("Failed to connect: {}", e),
            })?;
        
        let mut guard = self.client.lock().await;
        *guard = Some(client);
        Ok(())
    }

    pub async fn mutation(
        &self,
        function_name: String,
        args: Vec<(String, ConvexValue)>,
    ) -> Result<ConvexValue, ConvexError> {
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

        let result = client
            .mutation(&function_name, btree_args)
            .await
            .map_err(|e| ConvexError {
                message: format!("Mutation failed: {}", e),
            })?;

        let result_string = match result {
            FunctionResult::Value(val) => convex_value_to_json(val),
            FunctionResult::ErrorMessage(msg) => return Err(ConvexError { message: msg }),
            FunctionResult::ConvexError(err) => return Err(ConvexError { message: format!("Convex error: {:?}", err) }),
        };

        Ok(ConvexValue { inner: result_string })
    }

    pub async fn query(
        &self,
        function_name: String,
        args: Vec<(String, ConvexValue)>,
    ) -> Result<ConvexValue, ConvexError> {
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

        let result = client
            .query(&function_name, btree_args)
            .await
            .map_err(|e| ConvexError {
                message: format!("Query failed: {}", e),
            })?;

        let result_string = match result {
            FunctionResult::Value(val) => convex_value_to_json(val),
            FunctionResult::ErrorMessage(msg) => return Err(ConvexError { message: msg }),
            FunctionResult::ConvexError(err) => return Err(ConvexError { message: format!("Convex error: {:?}", err) }),
        };

        Ok(ConvexValue { inner: result_string })
    }
}

#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

// Helper functions for creating ConvexValue from common types
impl ConvexValue {
    #[frb(sync)]
    pub fn from_string(value: String) -> Self {
        let json_value = serde_json::Value::String(value);
        Self {
            inner: serde_json::to_string(&json_value).unwrap(),
        }
    }

    #[frb(sync)]
    pub fn from_int(value: i64) -> Self {
        let json_value = serde_json::Value::Number(serde_json::Number::from(value));
        Self {
            inner: serde_json::to_string(&json_value).unwrap(),
        }
    }

    #[frb(sync)]
    pub fn from_double(value: f64) -> Self {
        let json_value = serde_json::Value::Number(serde_json::Number::from_f64(value).unwrap());
        Self {
            inner: serde_json::to_string(&json_value).unwrap(),
        }
    }

    #[frb(sync)]
    pub fn from_bool(value: bool) -> Self {
        let json_value = serde_json::Value::Bool(value);
        Self {
            inner: serde_json::to_string(&json_value).unwrap(),
        }
    }

    #[frb(sync)]
    pub fn null() -> Self {
        Self {
            inner: "null".to_string(),
        }
    }
}