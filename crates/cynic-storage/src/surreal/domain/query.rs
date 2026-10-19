//! Functional Core: Pure SurrealQL query builders and statements.
//!
//! ZERO I/O: pure string manipulation and validation.

use serde::Serialize;

/// Pure query specification for SurrealQL operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurrealQuery {
    raw_sql: String,
}

impl SurrealQuery {
    pub fn raw(sql: impl Into<String>) -> Self {
        Self {
            raw_sql: sql.into(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.raw_sql
    }

    /// Pure builder for creating a record in a table.
    pub fn create_record<T: Serialize>(
        table: &str,
        id: Option<&str>,
        content: &T,
    ) -> Result<Self, String> {
        let json = serde_json::to_string(content)
            .map_err(|e| format!("failed to serialize content: {e}"))?;

        let target = match id {
            Some(i) => format!("{table}:{i}"),
            None => table.to_string(),
        };

        Ok(Self {
            raw_sql: format!("CREATE {target} CONTENT {json};"),
        })
    }

    /// Pure builder for selecting records from a table.
    pub fn select_all(table: &str) -> Self {
        Self {
            raw_sql: format!("SELECT * FROM {table};"),
        }
    }

    /// Pure builder for selecting by specific ID.
    pub fn select_by_id(table: &str, id: &str) -> Self {
        Self {
            raw_sql: format!("SELECT * FROM {table}:{id};"),
        }
    }

    /// Pure builder for database information probe.
    pub fn info_for_db() -> Self {
        Self {
            raw_sql: "INFO FOR DB;".to_string(),
        }
    }
}
