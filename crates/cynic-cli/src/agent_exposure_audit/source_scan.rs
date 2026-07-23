use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::rust_source::rust_files_under;

pub(crate) fn scan_mcp_tool_names(src_root: &Path) -> BTreeSet<String> {
    let mut tools = BTreeSet::new();
    for path in rust_files_under(src_root) {
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        tools.extend(extract_tool_names(&source));
    }
    tools
}

pub(crate) fn extract_tool_names(source: &str) -> BTreeSet<String> {
    let mut tools = BTreeSet::new();
    let bytes = source.as_bytes();
    let mut idx = 0usize;
    while let Some(rel) = source[idx..].find("#[tool(") {
        let start = idx + rel;
        let window = &source[start..source.len().min(start + 400)];
        if let Some(name) = parse_tool_name(window) {
            tools.insert(name);
        }
        idx = start + 7;
        if idx >= bytes.len() {
            break;
        }
    }
    tools
}

fn parse_tool_name(window: &str) -> Option<String> {
    let name_key = "name";
    let mut rest = window;
    while let Some(pos) = rest.find(name_key) {
        let after = rest[pos + name_key.len()..].trim_start();
        if !after.starts_with('=') {
            rest = &rest[pos + name_key.len()..];
            continue;
        }
        let after_eq = after[1..].trim_start();
        if let Some(stripped) = after_eq.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                return Some(stripped[..end].to_string());
            }
        }
        rest = after;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tool_names_from_attribute_blocks() {
        let source = r#"
            #[tool(
                name = "cynic_health",
                description = "health"
            )]
            async fn health() {}

            #[tool(name = "cynic_judge", description = "judge")]
            async fn judge() {}
        "#;
        let names = extract_tool_names(source);
        assert!(names.contains("cynic_health"));
        assert!(names.contains("cynic_judge"));
        assert_eq!(names.len(), 2);
    }
}
