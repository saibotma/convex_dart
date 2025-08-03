use std::collections::BTreeMap;
use convex::{ConvexClient, Value, FunctionResult};
use flutter_rust_bridge::frb;
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct ConvexClientWrapper {
    pub(crate) client: Arc<Mutex<Option<ConvexClient>>>,
}

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

#[derive(Debug, Clone)]
pub enum ConvexValue {
    Null,
    String(String),
    Int64(i64),
    Float64(f64),
    Array(Vec<ConvexValue>),
    Object(std::collections::HashMap<String, ConvexValue>),
    Bytes(Vec<u8>),
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
            // Convert our ConvexValue back to a String for Convex API
            // This is a temporary approach - ideally we'd convert directly to convex::Value
            let string_val = match value {
                ConvexValue::String(s) => s,
                ConvexValue::Int64(i) => i.to_string(),
                ConvexValue::Float64(f) => f.to_string(),
                ConvexValue::Null => "null".to_string(),
                _ => value.to_json_string(), // Fallback to JSON string
            };
            btree_args.insert(key, Value::String(string_val));
        }

        let result = client
            .mutation(&function_name, btree_args)
            .await
            .map_err(|e| ConvexError {
                message: format!("Mutation failed: {}", e),
            })?;

        let result_value = match result {
            FunctionResult::Value(val) => convert_value(val),
            FunctionResult::ErrorMessage(msg) => return Err(ConvexError { message: msg }),
            FunctionResult::ConvexError(err) => return Err(ConvexError { message: format!("Convex error: {:?}", err) }),
        };

        Ok(result_value)
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
            // Convert our ConvexValue back to a String for Convex API
            // This is a temporary approach - ideally we'd convert directly to convex::Value
            let string_val = match value {
                ConvexValue::String(s) => s,
                ConvexValue::Int64(i) => i.to_string(),
                ConvexValue::Float64(f) => f.to_string(),
                ConvexValue::Null => "null".to_string(),
                _ => value.to_json_string(), // Fallback to JSON string
            };
            btree_args.insert(key, Value::String(string_val));
        }

        let result = client
            .query(&function_name, btree_args)
            .await
            .map_err(|e| ConvexError {
                message: format!("Query failed: {}", e),
            })?;

        let result_value = match result {
            FunctionResult::Value(val) => convert_value(val),
            FunctionResult::ErrorMessage(msg) => return Err(ConvexError { message: msg }),
            FunctionResult::ConvexError(err) => return Err(ConvexError { message: format!("Convex error: {:?}", err) }),
        };

        Ok(result_value)
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
        ConvexValue::String(value)
    }

    #[frb(sync)]
    pub fn from_int(value: i64) -> Self {
        ConvexValue::Int64(value)
    }

    #[frb(sync)]
    pub fn from_double(value: f64) -> Self {
        ConvexValue::Float64(value)
    }

    #[frb(sync)]
    pub fn from_bool(value: bool) -> Self {
        // Since Convex doesn't have a Bool variant, we'll store it as a string
        ConvexValue::String(value.to_string())
    }

    #[frb(sync)]
    pub fn null_value() -> Self {
        ConvexValue::Null
    }
    
    // Helper method to convert to JSON string if needed for compatibility
    pub fn to_json_string(&self) -> String {
        match self {
            ConvexValue::Null => "null".to_string(),
            ConvexValue::String(s) => serde_json::to_string(s).unwrap_or_else(|_| format!("\"{}\"", s)),
            ConvexValue::Int64(i) => i.to_string(),
            ConvexValue::Float64(f) => f.to_string(),
            ConvexValue::Array(arr) => {
                let json_items: Vec<String> = arr.iter().map(|v| v.to_json_string()).collect();
                format!("[{}]", json_items.join(","))
            },
            ConvexValue::Object(obj) => {
                let json_pairs: Vec<String> = obj.iter()
                    .map(|(k, v)| format!("\"{}\":{}", k, v.to_json_string()))
                    .collect();
                format!("{{{}}}", json_pairs.join(","))
            },
            ConvexValue::Bytes(_) => "null".to_string(),
        }
    }
}