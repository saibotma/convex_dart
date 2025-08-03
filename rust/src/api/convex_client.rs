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
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => serde_json::to_string(&s).unwrap_or_else(|_| format!("\"{}\"", s)),
        Value::Int64(i) => i.to_string(),
        Value::Float64(f) => f.to_string(),
        Value::Array(arr) => {
            let json_array: Vec<serde_json::Value> = arr.into_iter().map(|v| {
                match convex_value_to_serde_json(v) {
                    Ok(json_val) => json_val,
                    Err(_) => serde_json::Value::Null,
                }
            }).collect();
            serde_json::to_string(&json_array).unwrap_or_else(|_| "[]".to_string())
        },
        Value::Object(obj) => {
            let json_obj: serde_json::Map<String, serde_json::Value> = obj.into_iter().map(|(k, v)| {
                let json_val = match convex_value_to_serde_json(v) {
                    Ok(val) => val,
                    Err(_) => serde_json::Value::Null,
                };
                (k, json_val)
            }).collect();
            serde_json::to_string(&serde_json::Value::Object(json_obj)).unwrap_or_else(|_| "{}".to_string())
        },
        Value::Bytes(_) => "null".to_string(), // Handle bytes as null for simplicity
    }
}

// Helper function to convert Convex Value to serde_json::Value
fn convex_value_to_serde_json(value: Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match value {
        Value::Null => Ok(serde_json::Value::Null),
        Value::Bool(b) => Ok(serde_json::Value::Bool(b)),
        Value::String(s) => Ok(serde_json::Value::String(s)),
        Value::Int64(i) => Ok(serde_json::Value::Number(serde_json::Number::from(i))),
        Value::Float64(f) => {
            if let Some(num) = serde_json::Number::from_f64(f) {
                Ok(serde_json::Value::Number(num))
            } else {
                Ok(serde_json::Value::Null)
            }
        },
        Value::Array(arr) => {
            let json_array: Result<Vec<serde_json::Value>, _> = arr.into_iter()
                .map(convex_value_to_serde_json)
                .collect();
            Ok(serde_json::Value::Array(json_array?))
        },
        Value::Object(obj) => {
            let mut json_obj = serde_json::Map::new();
            for (k, v) in obj.into_iter() {
                json_obj.insert(k, convex_value_to_serde_json(v)?);
            }
            Ok(serde_json::Value::Object(json_obj))
        },
        Value::Bytes(_) => Ok(serde_json::Value::Null), // Handle bytes as null for simplicity
    }
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