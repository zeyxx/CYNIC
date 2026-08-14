//! Validation and parsing helpers for SurrealQL queries.

use crate::domain::storage::StorageError;

/// Validate IDs: alphanumeric, hyphens, underscores only. Max 128 chars.
pub(crate) fn sanitize_id(id: &str) -> Result<&str, StorageError> {
    if id.is_empty() || id.len() > 128 {
        return Err(StorageError::QueryFailed(
            "ID must be 1-128 characters".into(),
        ));
    }

    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(StorageError::QueryFailed(
            "ID contains invalid characters".into(),
        ));
    }

    Ok(id)
}

/// Escape string for SurrealQL string literals.
pub(crate) fn escape_surreal(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('`', "\\`")
        .replace('\0', "")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Clamp query limit to prevent resource exhaustion.
pub(crate) fn safe_limit(limit: u32) -> u32 {
    limit.min(100)
}

pub(crate) fn parse_sql_table_op(sql: &str) -> Option<(String, String)> {
    let normalized = sql.trim().to_uppercase();

    let ops = [
        "CREATE", "SELECT", "UPDATE", "DELETE", "INSERT", "UPSERT", "REMOVE", "DEFINE",
    ];

    let (operation, rest) = match ops.iter().find(|op| normalized.starts_with(*op)) {
        Some(op) => (*op, normalized[op.len()..].trim_start().to_string()),
        None => return None,
    };

    let mut table_chars = Vec::new();
    let mut in_backtick = false;

    for ch in rest.chars() {
        if ch == '`' {
            in_backtick = !in_backtick;
        } else if in_backtick || ch.is_ascii_alphanumeric() || ch == '_' {
            table_chars.push(ch);
        } else if !ch.is_whitespace() || table_chars.is_empty() {
            continue;
        } else {
            break;
        }
    }

    let table = table_chars.iter().collect::<String>();
    if table.is_empty() {
        return None;
    }

    Some((table, operation.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_surreal_handles_backticks() {
        assert_eq!(escape_surreal("hello`world"), "hello\\`world");
    }

    #[test]
    fn escape_surreal_handles_quotes_and_backslashes() {
        assert_eq!(escape_surreal("it's a \\test"), "it\\'s a \\\\test");
    }

    #[test]
    fn escape_surreal_handles_null_bytes() {
        assert_eq!(escape_surreal("a\0b"), "ab");
    }

    #[test]
    fn sanitize_id_rejects_empty() {
        assert!(sanitize_id("").is_err());
    }

    #[test]
    fn sanitize_id_rejects_long() {
        let long = "a".repeat(129);
        assert!(sanitize_id(&long).is_err());
    }

    #[test]
    fn sanitize_id_allows_valid() {
        assert!(sanitize_id("my-dog_123").is_ok());
    }

    #[test]
    fn sanitize_id_rejects_special_chars() {
        assert!(sanitize_id("dog'; DROP TABLE--").is_err());
    }
}
