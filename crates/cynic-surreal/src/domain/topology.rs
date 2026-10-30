//! Topological context for graph-first RAG retrieval.
//!
//! Pure domain logic: struct definition and parsing, zero I/O.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Topological context for a chunk, derived from graph traversal in SurrealDB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologicalContext {
    pub parent_path: String,
    pub parent_authority: String,
    pub enforces: Vec<String>,
    pub depends_on: Vec<String>,
    pub cites: Vec<String>,
    pub enforced_by: Vec<String>,
    pub depended_on_by: Vec<String>,
}

impl TopologicalContext {
    pub const SCHEMA: &'static str = "cynic.surreal.topological_context.v1";

    /// Build the SurrealQL query for fetching topological context for given chunk IDs.
    pub fn fetch_query(chunk_ids: &[String]) -> String {
        let ids_json = serde_json::to_string(chunk_ids).unwrap_or_else(|_| "[]".to_string());
        format!(
            "SELECT id, \
             <-contains_chunk<-doc.path AS parent_path, \
             <-contains_chunk<-doc.authority AS parent_authority, \
             <-contains_chunk<-doc->enforces->doc.path AS enforces, \
             <-contains_chunk<-doc->depends_on->doc.path AS depends_on, \
             <-contains_chunk<-doc->cites->doc.path AS cites, \
             <-contains_chunk<-doc<-enforces<-doc.path AS enforced_by, \
             <-contains_chunk<-doc<-depends_on<-doc.path AS depended_on_by \
             FROM chunk WHERE id IN {ids_json};"
        )
    }

    /// Parse a single SurrealDB result row into a TopologicalContext.
    pub fn from_row(row: &serde_json::Value) -> Option<(String, Self)> {
        let id = row.get("id")?.as_str()?.to_string();
        let parent_path = row
            .get("parent_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let parent_authority = row
            .get("parent_authority")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let enforces = parse_string_array(row.get("enforces"));
        let depends_on = parse_string_array(row.get("depends_on"));
        let cites = parse_string_array(row.get("cites"));
        let enforced_by = parse_string_array(row.get("enforced_by"));
        let depended_on_by = parse_string_array(row.get("depended_on_by"));

        Some((
            id,
            Self {
                parent_path,
                parent_authority,
                enforces,
                depends_on,
                cites,
                enforced_by,
                depended_on_by,
            },
        ))
    }

    /// Parse a list of SurrealDB result rows into a HashMap.
    pub fn from_rows(rows: &[serde_json::Value]) -> HashMap<String, Self> {
        let mut map = HashMap::new();
        for row in rows {
            if let Some((id, ctx)) = Self::from_row(row) {
                map.insert(id, ctx);
            }
        }
        map
    }
}

fn parse_string_array(val: Option<&serde_json::Value>) -> Vec<String> {
    match val {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        Some(serde_json::Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_query_contains_chunk_ids() {
        let ids = vec!["chunk:abc".to_string(), "chunk:def".to_string()];
        let query = TopologicalContext::fetch_query(&ids);
        assert!(query.contains("chunk:abc"));
        assert!(query.contains("chunk:def"));
        assert!(query.contains("FROM chunk WHERE id IN"));
    }

    #[test]
    fn from_row_parses_correctly() {
        let row = serde_json::json!({
            "id": "chunk:abc",
            "parent_path": "docs/test.md",
            "parent_authority": "L1Specification",
            "enforces": ["docs/a.md"],
            "depends_on": ["docs/b.md"],
            "cites": ["docs/c.md"],
            "enforced_by": [],
            "depended_on_by": ["docs/d.md"]
        });
        let (id, ctx) = TopologicalContext::from_row(&row).unwrap();
        assert_eq!(id, "chunk:abc");
        assert_eq!(ctx.parent_path, "docs/test.md");
        assert_eq!(ctx.enforces, vec!["docs/a.md"]);
        assert_eq!(ctx.depended_on_by, vec!["docs/d.md"]);
    }

    #[test]
    fn from_rows_builds_map() {
        let rows = vec![
            serde_json::json!({"id": "chunk:1", "parent_path": "a.md", "parent_authority": "L0", "enforces": [], "depends_on": [], "cites": [], "enforced_by": [], "depended_on_by": []}),
            serde_json::json!({"id": "chunk:2", "parent_path": "b.md", "parent_authority": "L1", "enforces": [], "depends_on": [], "cites": [], "enforced_by": [], "depended_on_by": []}),
        ];
        let map = TopologicalContext::from_rows(&rows);
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("chunk:1"));
        assert!(map.contains_key("chunk:2"));
    }
}