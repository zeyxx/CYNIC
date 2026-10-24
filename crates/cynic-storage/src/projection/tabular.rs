//! In-memory and ephemeral SQLite tabular projections for zero-I/O querying of validated datasets.

use std::time::Instant;
use rusqlite::{Connection, params_from_iter, types::ToSql};
use serde::{Deserialize, Serialize};

use cynic_protocol::tabular::{DataColumnType, TabularSchema};
use crate::error::StorageError;

/// Dynamic query result containing projected column headers, typed values, and execution metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabularQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub elapsed_ms: u64,
}

/// In-memory SQLite engine for executing SQL queries on pre-validated tabular data records.
pub struct TabularMemoryProjection {
    conn: Connection,
    table_name: String,
    schema: TabularSchema,
}

impl std::fmt::Debug for TabularMemoryProjection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabularMemoryProjection")
            .field("table_name", &self.table_name)
            .field("schema", &self.schema)
            .finish()
    }
}

impl TabularMemoryProjection {
    /// Initializes an in-memory SQLite database, creates the typed table according to the given schema,
    /// and generates a canonical alias view named `data` for simplified querying.
    pub fn create_from_schema(
        schema: TabularSchema,
        table_name_override: Option<&str>,
    ) -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| StorageError::Io(format!("failed to open in-memory SQLite: {e}")))?;

        // Sanitized table name from schema_id if not overridden
        let table_name = match table_name_override {
            Some(name) => sanitize_identifier(name),
            None => {
                // e.g. "cynic.session_summary.v1" -> "session_summary"
                extract_short_table_name(&schema.schema_id)
            }
        };

        // Build CREATE TABLE DDL
        let ddl = generate_create_table_ddl(&table_name, &schema);
        conn.execute_batch(&ddl)
            .map_err(|e| StorageError::Io(format!("failed to create table '{table_name}': {e}")))?;

        // If the table is not already named 'data', create a canonical view 'data'
        if table_name != "data" {
            let view_ddl = format!("CREATE VIEW IF NOT EXISTS data AS SELECT * FROM {table_name};");
            conn.execute_batch(&view_ddl)
                .map_err(|e| StorageError::Io(format!("failed to create 'data' view: {e}")))?;
        }

        Ok(Self {
            conn,
            table_name,
            schema,
        })
    }

    /// Returns the target table name.
    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    /// Creates an additional table in the in-memory SQLite database according to the given schema.
    pub fn add_table_from_schema(
        &mut self,
        schema: TabularSchema,
        table_name_override: Option<&str>,
    ) -> Result<String, StorageError> {
        let table_name = match table_name_override {
            Some(name) => sanitize_identifier(name),
            None => extract_short_table_name(&schema.schema_id),
        };

        let ddl = generate_create_table_ddl(&table_name, &schema);
        self.conn
            .execute_batch(&ddl)
            .map_err(|e| StorageError::Io(format!("failed to create table '{table_name}': {e}")))?;

        Ok(table_name)
    }

    /// Inserts a slice of validated records into a specific named table matching a given schema.
    pub fn insert_records_into_table(
        &mut self,
        table_name: &str,
        schema: &TabularSchema,
        records: &[serde_json::Map<String, serde_json::Value>],
    ) -> Result<usize, StorageError> {
        if records.is_empty() {
            return Ok(0);
        }

        let column_names: Vec<String> = schema
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect();

        let placeholders = column_names
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");

        let insert_sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            sanitize_identifier(table_name),
            column_names.join(", "),
            placeholders
        );

        let tx = self
            .conn
            .transaction()
            .map_err(|e| StorageError::Io(format!("failed to begin SQLite transaction: {e}")))?;

        let mut inserted = 0;
        {
            let mut stmt = tx
                .prepare(&insert_sql)
                .map_err(|e| StorageError::Io(format!("failed to prepare insert statement: {e}")))?;

            for record in records {
                let mut params: Vec<Box<dyn ToSql>> = Vec::with_capacity(column_names.len());

                for col in &schema.columns {
                    params.push(json_value_to_tosql(record.get(&col.name), col.data_type));
                }

                stmt.execute(params_from_iter(params.iter().map(|b| b.as_ref())))
                    .map_err(|e| StorageError::Io(format!("failed to insert row into '{table_name}': {e}")))?;
                inserted += 1;
            }
        }

        tx.commit()
            .map_err(|e| StorageError::Io(format!("failed to commit insert transaction: {e}")))?;

        Ok(inserted)
    }

    /// Inserts a slice of validated records (column name -> JSON value) into the SQLite table.
    pub fn insert_records(
        &mut self,
        records: &[serde_json::Map<String, serde_json::Value>],
    ) -> Result<usize, StorageError> {
        let table_name = self.table_name.clone();
        let schema = self.schema.clone();
        self.insert_records_into_table(&table_name, &schema, records)
    }

    /// Executes an arbitrary SQL query against the SQLite in-memory projection and returns dynamic results.
    pub fn query(&self, sql: &str) -> Result<TabularQueryResult, StorageError> {
        let start = Instant::now();
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| StorageError::Io(format!("SQL syntax/prepare error: {e}")))?;

        let col_count = stmt.column_count();
        let columns: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let mut rows = stmt
            .query([])
            .map_err(|e| StorageError::Io(format!("SQL execution error: {e}")))?;

        let mut result_rows = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| StorageError::Io(format!("error fetching row: {e}")))?
        {
            let mut row_values = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let val_ref = row
                    .get_ref(i)
                    .map_err(|e| StorageError::Io(format!("error reading column {i}: {e}")))?;

                let json_val = match val_ref {
                    rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                    rusqlite::types::ValueRef::Integer(i) => serde_json::json!(i),
                    rusqlite::types::ValueRef::Real(f) => serde_json::json!(f),
                    rusqlite::types::ValueRef::Text(t) => {
                        let text = std::str::from_utf8(t).unwrap_or("");
                        // If valid json array/object, leave it as structured JSON
                        if (text.starts_with('{') && text.ends_with('}'))
                            || (text.starts_with('[') && text.ends_with(']'))
                        {
                            serde_json::from_str(text).unwrap_or_else(|_| serde_json::json!(text))
                        } else {
                            serde_json::json!(text)
                        }
                    }
                    rusqlite::types::ValueRef::Blob(b) => serde_json::json!(hex::encode(b)),
                };
                row_values.push(json_val);
            }
            result_rows.push(row_values);
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;
        let row_count = result_rows.len();

        Ok(TabularQueryResult {
            columns,
            rows: result_rows,
            row_count,
            elapsed_ms,
        })
    }
}

/// Converts an optional JSON value into a boxed Rusqlite `ToSql` trait object according to the schema column type.
fn json_value_to_tosql(val: Option<&serde_json::Value>, col_type: DataColumnType) -> Box<dyn ToSql> {
    match val {
        None | Some(serde_json::Value::Null) => Box::new(rusqlite::types::Null),
        Some(serde_json::Value::Bool(b)) => Box::new(if *b { 1i64 } else { 0i64 }),
        Some(serde_json::Value::Number(n)) => match col_type {
            DataColumnType::Integer => Box::new(n.as_i64().unwrap_or(0)),
            DataColumnType::Float => Box::new(n.as_f64().unwrap_or(0.0)),
            _ => Box::new(n.to_string()),
        },
        Some(serde_json::Value::String(s)) => match col_type {
            DataColumnType::Integer => Box::new(s.parse::<i64>().unwrap_or(0)),
            DataColumnType::Float => Box::new(s.parse::<f64>().unwrap_or(0.0)),
            DataColumnType::Boolean => Box::new(if matches!(s.to_ascii_lowercase().as_str(), "true" | "1" | "yes") { 1i64 } else { 0i64 }),
            _ => Box::new(s.clone()),
        },
        Some(other) => Box::new(other.to_string()),
    }
}

/// Sanitizes a string to be a safe SQL table/column identifier.
fn sanitize_identifier(s: &str) -> String {
    let sanitized: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if sanitized.is_empty() || sanitized.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        format!("tbl_{sanitized}")
    } else {
        sanitized
    }
}

/// Extracts a clean short table name from a schema ID (e.g. "cynic.session_summary.v1" -> "session_summary").
fn extract_short_table_name(schema_id: &str) -> String {
    let parts: Vec<&str> = schema_id.split('.').collect();
    if parts.len() >= 3 {
        // e.g. ["cynic", "session_summary", "v1"]
        sanitize_identifier(parts[1])
    } else if let Some(last) = parts.last() {
        sanitize_identifier(last)
    } else {
        "tabular_data".to_string()
    }
}

/// Generates DDL `CREATE TABLE` statement matching the columns and types of the schema.
fn generate_create_table_ddl(table_name: &str, schema: &TabularSchema) -> String {
    let mut cols_ddl = Vec::new();

    for col in &schema.columns {
        let col_type = match col.data_type {
            DataColumnType::Integer => "INTEGER",
            DataColumnType::Float => "REAL",
            DataColumnType::Boolean => "INTEGER",
            DataColumnType::Text | DataColumnType::TimestampIso | DataColumnType::Json => "TEXT",
        };

        let not_null = if col.required { " NOT NULL" } else { "" };
        let col_ident = sanitize_identifier(&col.name);
        cols_ddl.push(format!("{col_ident} {col_type}{not_null}"));
    }

    format!(
        "CREATE TABLE IF NOT EXISTS {table_name} (\n  {}\n);",
        cols_ddl.join(",\n  ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::tabular::ColumnDefinition;

    #[test]
    fn test_tabular_memory_projection_ddl_and_query() {
        let schema = TabularSchema::new(
            "cynic.test_inventory.v1",
            1,
            vec![
                ColumnDefinition::new("node_id", DataColumnType::Text, true),
                ColumnDefinition::new("cores", DataColumnType::Integer, true),
                ColumnDefinition::new("ram_gb", DataColumnType::Float, false),
                ColumnDefinition::new("is_active", DataColumnType::Boolean, true),
            ],
        );

        let mut proj = TabularMemoryProjection::create_from_schema(schema, None).expect("create proj");
        assert_eq!(proj.table_name(), "test_inventory");

        let mut r1 = serde_json::Map::new();
        r1.insert("node_id".to_string(), serde_json::json!("node-cortex"));
        r1.insert("cores".to_string(), serde_json::json!(16));
        r1.insert("ram_gb".to_string(), serde_json::json!(64.5));
        r1.insert("is_active".to_string(), serde_json::json!(true));

        let mut r2 = serde_json::Map::new();
        r2.insert("node_id".to_string(), serde_json::json!("node-edge"));
        r2.insert("cores".to_string(), serde_json::json!(4));
        r2.insert("ram_gb".to_string(), serde_json::json!(16.0));
        r2.insert("is_active".to_string(), serde_json::json!(false));

        let inserted = proj.insert_records(&[r1, r2]).expect("insert");
        assert_eq!(inserted, 2);

        // Query direct table
        let q1 = proj.query("SELECT node_id, cores FROM test_inventory WHERE cores > 10").expect("query");
        assert_eq!(q1.columns, vec!["node_id", "cores"]);
        assert_eq!(q1.row_count, 1);
        assert_eq!(q1.rows[0][0], serde_json::json!("node-cortex"));
        assert_eq!(q1.rows[0][1], serde_json::json!(16));

        // Query canonical alias 'data' view
        let q2 = proj.query("SELECT count(*) as total, sum(cores) as total_cores FROM data").expect("query view");
        assert_eq!(q2.columns, vec!["total", "total_cores"]);
        assert_eq!(q2.row_count, 1);
        assert_eq!(q2.rows[0][0], serde_json::json!(2));
        assert_eq!(q2.rows[0][1], serde_json::json!(20));
    }

    #[test]
    fn test_tabular_memory_projection_multi_table_join() {
        let parent_schema = TabularSchema::new(
            "cynic.parent.v1",
            1,
            vec![
                ColumnDefinition::new("id", DataColumnType::Text, true),
                ColumnDefinition::new("name", DataColumnType::Text, true),
            ],
        );
        let child_schema = TabularSchema::new(
            "cynic.child.v1",
            1,
            vec![
                ColumnDefinition::new("child_id", DataColumnType::Text, true),
                ColumnDefinition::new("parent_id", DataColumnType::Text, true),
                ColumnDefinition::new("score", DataColumnType::Integer, true),
            ],
        );

        let mut proj = TabularMemoryProjection::create_from_schema(parent_schema, Some("parents"))
            .expect("create proj");
        proj.add_table_from_schema(child_schema.clone(), Some("children"))
            .expect("add table");

        let mut p1 = serde_json::Map::new();
        p1.insert("id".to_string(), serde_json::json!("p1"));
        p1.insert("name".to_string(), serde_json::json!("Alice"));
        proj.insert_records(&[p1]).expect("insert parent");

        let mut c1 = serde_json::Map::new();
        c1.insert("child_id".to_string(), serde_json::json!("c1"));
        c1.insert("parent_id".to_string(), serde_json::json!("p1"));
        c1.insert("score".to_string(), serde_json::json!(42));
        proj.insert_records_into_table("children", &child_schema, &[c1])
            .expect("insert child");

        let res = proj
            .query("SELECT p.name, c.score FROM parents p JOIN children c ON p.id = c.parent_id")
            .expect("query join");
        assert_eq!(res.columns, vec!["name", "score"]);
        assert_eq!(res.row_count, 1);
        assert_eq!(res.rows[0][0], serde_json::json!("Alice"));
        assert_eq!(res.rows[0][1], serde_json::json!(42));
    }
}
