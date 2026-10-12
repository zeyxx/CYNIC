//! Functional Core: Pure response parser and validator for SurrealQL execution.
//!
//! ZERO I/O: pure data transformations and assertions.

use serde::{Deserialize, Serialize};

/// Generic item response envelope from SurrealDB `/sql`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurrealQueryResult {
    pub status: String,
    pub time: Option<String>,
    pub result: Option<serde_json::Value>,
    pub details: Option<serde_json::Value>,
}

impl SurrealQueryResult {
    pub fn is_ok(&self) -> bool {
        self.status == "OK"
    }

    pub fn error_message(&self) -> Option<String> {
        if self.is_ok() {
            None
        } else {
            self.result
                .as_ref()
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or_else(|| self.details.as_ref().map(|d| d.to_string()))
        }
    }
}

/// Pure validation of query results returning unwrapped payload.
pub fn parse_and_validate_results(
    results: Vec<SurrealQueryResult>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut values = Vec::new();
    for res in results {
        if !res.is_ok() {
            return Err(format!(
                "SurrealQL error: {}",
                res.error_message()
                    .unwrap_or_else(|| "unknown error".into())
            ));
        }
        if let Some(val) = res.result {
            values.push(val);
        }
    }
    Ok(values)
}
