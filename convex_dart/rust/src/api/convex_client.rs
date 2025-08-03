use std::collections::BTreeMap;
use convex::{ConvexClient, Value, FunctionResult};
use flutter_rust_bridge::frb;
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct ConvexClientWrapper {
    pub(crate) client: Arc<Mutex<Option<ConvexClient>>>,
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
            FunctionResult::Value(val) => match val {
                Value::String(s) => format!("\"{}\"", s),
                _ => format!("{:?}", val),
            },
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
            FunctionResult::Value(val) => match val {
                Value::String(s) => format!("\"{}\"", s),
                _ => format!("{:?}", val),
            },
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