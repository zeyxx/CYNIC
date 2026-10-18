//! Conservation contract — the alpha every harness must satisfy.
//!
//! Strict port of `apps/cynic-data/src/cynic_data/canonical/domain/conservation.py`.
//! Pure: no I/O, no producer knowledge. Producers supply a contract.

use std::collections::{HashMap, HashSet};

use regex::Regex;
use serde::{Deserialize, Serialize};

/// The two, and only two, things that can happen to a raw field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fate {
    Preserved,
    Declared,
}

/// Split `"preserved: turn.content"` into `(Fate::Preserved, "turn.content")`.
/// A fate without a known prefix is a contract bug: error, never guess.
pub fn parse_fate(value: &str) -> Result<(Fate, String), String> {
    let (prefix, detail) = match value.split_once(':') {
        Some((p, d)) => (p.trim(), d.trim().to_owned()),
        None => {
            return Err(format!(
                "fate must start with preserved:/declared:, got {value:?}"
            ));
        }
    };
    let fate = match prefix {
        "preserved" => Fate::Preserved,
        "declared" => Fate::Declared,
        _ => {
            return Err(format!(
                "fate must start with preserved:/declared:, got {value:?}"
            ));
        }
    };
    Ok((fate, detail))
}

/// How one concept is realised across a producer's models.
/// `homes` is priority-ordered: the first non-null column wins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConceptResolution {
    pub concept: String,
    pub homes: Vec<String>,
    pub model_profiles: HashMap<String, String>,
}

impl ConceptResolution {
    /// Return `(home, value)` for the first non-null home, else `(None, None)`.
    pub fn resolve(
        &self,
        row: &HashMap<String, serde_json::Value>,
    ) -> (Option<String>, Option<serde_json::Value>) {
        for home in &self.homes {
            match row.get(home) {
                Some(v) if !v.is_null() => return (Some(home.clone()), Some(v.clone())),
                _ => continue,
            }
        }
        (None, None)
    }

    /// The home this model was observed to use, or None if unknown.
    /// Longest-pattern-first so specific models beat family prefixes.
    pub fn expected_home(&self, model: Option<&str>) -> Option<String> {
        let model = model?;
        if let Some(home) = self.model_profiles.get(model) {
            return Some(home.clone());
        }
        let mut patterns: Vec<&String> = self.model_profiles.keys().collect();
        patterns.sort_by_key(|p| std::cmp::Reverse(p.len()));
        for pattern in patterns {
            if !pattern.is_empty() && model.contains(pattern.as_str()) {
                return self.model_profiles.get(pattern).cloned();
            }
        }
        None
    }
}

/// The guarantee a normalizer makes about a producer's raw store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConservationContract {
    pub producer: String,
    pub producer_schema_version: i64,
    pub producer_schema_source: String,
    pub field_fate: HashMap<String, String>,
    pub concepts: Vec<ConceptResolution>,
    /// Canonical home -> human description of the transform applied on the way in.
    pub transforms: HashMap<String, String>,
    /// Producer tables deliberately NOT modeled, each with a reason.
    pub out_of_scope: HashMap<String, String>,
}

impl ConservationContract {
    /// Observed raw fields with no declared fate. Must be empty.
    pub fn missing_fates(&self, observed_fields: &HashSet<String>) -> Vec<String> {
        let mut missing: Vec<String> = observed_fields
            .iter()
            .filter(|f| !self.field_fate.contains_key(*f))
            .cloned()
            .collect();
        missing.sort();
        missing
    }

    /// Tables NEITHER covered by a fate map NOR named out of scope. Must be empty.
    pub fn unmodeled_tables(&self, observed: &HashMap<String, Vec<String>>) -> Vec<String> {
        let fated_tables: HashSet<String> = self
            .field_fate
            .keys()
            .filter_map(|k| k.split_once('.').map(|(t, _)| t.to_owned()))
            .collect();
        let mut unmodeled: Vec<String> = observed
            .keys()
            .filter(|t| !self.out_of_scope.contains_key(*t) && !fated_tables.contains(*t))
            .cloned()
            .collect();
        unmodeled.sort();
        unmodeled
    }

    /// Tables declared out of scope that the producer no longer has.
    pub fn stale_out_of_scope(&self, observed: &HashMap<String, Vec<String>>) -> Vec<String> {
        let mut stale: Vec<String> = self
            .out_of_scope
            .keys()
            .filter(|t| !observed.contains_key(*t))
            .cloned()
            .collect();
        stale.sort();
        stale
    }

    /// Columns of IN-SCOPE tables with no fate.
    pub fn uncovered_columns(
        &self,
        observed: &HashMap<String, Vec<String>>,
    ) -> HashMap<String, Vec<String>> {
        let mut holes = HashMap::new();
        for (table, cols) in observed {
            if self.out_of_scope.contains_key(table) {
                continue;
            }
            let missing: Vec<String> = cols
                .iter()
                .filter(|c| !self.field_fate.contains_key(&format!("{table}.{c}")))
                .cloned()
                .collect();
            if !missing.is_empty() {
                holes.insert(table.clone(), {
                    let mut m = missing;
                    m.sort();
                    m
                });
            }
        }
        holes
    }

    /// Fates that do not start with preserved:/declared:.
    pub fn malformed_fates(&self) -> Vec<String> {
        let mut bad: Vec<String> = self
            .field_fate
            .iter()
            .filter(|(_, v)| parse_fate(v).is_err())
            .map(|(k, v)| format!("{k} -> {v}"))
            .collect();
        bad.sort();
        bad
    }

    pub fn preserved_fields(&self) -> HashMap<String, String> {
        self.field_fate
            .iter()
            .filter_map(|(k, v)| match parse_fate(v) {
                Ok((Fate::Preserved, detail)) => Some((k.clone(), detail)),
                _ => None,
            })
            .collect()
    }

    pub fn declared_losses(&self) -> HashMap<String, String> {
        self.field_fate
            .iter()
            .filter_map(|(k, v)| match parse_fate(v) {
                Ok((Fate::Declared, detail)) => Some((k.clone(), detail)),
                _ => None,
            })
            .collect()
    }

    pub fn concept(&self, name: &str) -> Option<&ConceptResolution> {
        self.concepts.iter().find(|c| c.concept == name)
    }
}

static NOT_A_COLUMN: &[&str] = &[
    "primary",
    "foreign",
    "unique",
    "check",
    "constraint",
    "references",
    "not",
    "null",
    "default",
    "on",
    "delete",
    "cascade",
    "autoincrement",
];

/// Extract `{table: [column, ...]}` from a CREATE TABLE script.
/// The producer's DDL is the source of truth, never a hand-kept list.
pub fn parse_ddl_columns(ddl: &str) -> HashMap<String, Vec<String>> {
    let re = Regex::new(r"(?is)CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(\w+)\s*\((.*?)\)\s*;")
        .expect("static DDL regex");
    let mut tables = HashMap::new();
    for cap in re.captures_iter(ddl) {
        let table = cap[1].to_lowercase();
        let body = &cap[2];
        let mut cols = Vec::new();
        let mut depth: i64 = 0;
        for raw_line in body.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with("--") {
                continue;
            }
            depth += line.matches('(').count() as i64 - line.matches(')').count() as i64;
            let first = line
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(',')
                .to_lowercase();
            if depth <= 0 && NOT_A_COLUMN.contains(&first.as_str()) {
                continue;
            }
            let name = line.split_whitespace().next().unwrap_or("").trim_matches(',');
            if !name.is_empty()
                && !NOT_A_COLUMN.contains(&name.to_lowercase().as_str())
                && !name.starts_with("--")
            {
                cols.push(name.to_owned());
            }
        }
        tables.insert(table, cols);
    }
    tables
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fate_parses_and_rejects() {
        assert_eq!(
            parse_fate("preserved: turn.content"),
            Ok((Fate::Preserved, "turn.content".into()))
        );
        assert_eq!(
            parse_fate("declared: dropped on purpose"),
            Ok((Fate::Declared, "dropped on purpose".into()))
        );
        assert!(parse_fate("turn.content").is_err());
        assert!(parse_fate("maybe: turn.content").is_err());
    }

    #[test]
    fn resolution_is_priority_ordered() {
        let c = ConceptResolution {
            concept: "reasoning".into(),
            homes: vec!["b".into(), "a".into()],
            model_profiles: [("m-long".to_owned(), "b".to_owned())].into(),
        };
        let row: HashMap<String, serde_json::Value> = [
            ("a".to_owned(), serde_json::Value::from(1)),
            ("b".to_owned(), serde_json::Value::from(2)),
        ]
        .into();
        assert_eq!(c.resolve(&row).0.as_deref(), Some("b"));
        assert_eq!(c.expected_home(Some("m-long-x")), Some("b".to_owned()));
        assert_eq!(c.expected_home(Some("unknown")), None);
        let empty: HashMap<String, serde_json::Value> = HashMap::new();
        assert_eq!(c.resolve(&empty), (None, None));
    }

    #[test]
    fn closure_methods_agree() {
        let c = ConservationContract {
            producer: "p".into(),
            producer_schema_version: 1,
            producer_schema_source: "x".into(),
            field_fate: [("t.a".to_owned(), "preserved: turn.a".to_owned())].into(),
            concepts: vec![],
            transforms: HashMap::new(),
            out_of_scope: [("ops".to_owned(), "infra, not history".to_owned())].into(),
        };
        let observed: HashMap<String, Vec<String>> = [
            ("t".to_owned(), vec!["a".to_owned(), "b".to_owned()]),
            ("ops".to_owned(), vec!["x".to_owned()]),
        ]
        .into();
        let fields: HashSet<String> =
            ["t.a".to_owned(), "t.b".to_owned(), "ops.x".to_owned()].into();
        assert_eq!(c.missing_fates(&fields), vec!["ops.x".to_owned(), "t.b".to_owned()]);
        assert!(c.unmodeled_tables(&observed).is_empty());
        assert!(c.stale_out_of_scope(&observed).is_empty());
        assert_eq!(
            c.uncovered_columns(&observed).get("t"),
            Some(&vec!["b".to_owned()])
        );
        assert!(c.malformed_fates().is_empty());
    }

    #[test]
    fn ddl_parser_reads_tables() {
        let ddl = "CREATE TABLE messages (\n id INTEGER PRIMARY KEY,\n session_id TEXT,\n content TEXT\n);";
        let tables = parse_ddl_columns(ddl);
        assert_eq!(
            tables.get("messages"),
            Some(&vec!["id".to_owned(), "session_id".to_owned(), "content".to_owned()])
        );
    }
}
