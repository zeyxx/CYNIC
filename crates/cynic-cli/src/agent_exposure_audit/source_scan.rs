use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::rust_source::rust_files_under;

#[derive(Debug, Clone)]
pub(crate) struct McpToolDiscovery {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) line: usize,
}

pub(crate) fn scan_mcp_tool_names(src_root: &Path) -> BTreeSet<String> {
    scan_mcp_tools(src_root)
        .into_iter()
        .map(|t| t.name)
        .collect()
}

pub(crate) fn scan_mcp_tools(src_root: &Path) -> Vec<McpToolDiscovery> {
    let mut tools = Vec::new();
    for path in rust_files_under(src_root) {
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        tools.extend(extract_tools(&source));
    }
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    tools
}

fn extract_tools(source: &str) -> Vec<McpToolDiscovery> {
    let mut tools = Vec::new();
    let mut idx = 0;
    while let Some(rel) = source[idx..].find("#[tool(") {
        let start = idx + rel;
        let end = source[start..].find(']').unwrap_or(source.len() - start);
        let block = &source[start..start + end];
        let name = extract_attr_value(block, "name");
        let description = extract_attr_value(block, "description");
        if let Some(name) = name {
            let line = source[..start].matches('\n').count() + 1;
            tools.push(McpToolDiscovery {
                name,
                description: description.unwrap_or_default(),
                line,
            });
        }
        idx = start + 7;
        if idx >= source.len() {
            break;
        }
    }
    tools
}

fn extract_attr_value(block: &str, key: &str) -> Option<String> {
    if let Some(pos) = block.find(key) {
        let after = &block[pos + key.len()..];
        let after = after.trim_start();
        if let Some(after) = after.strip_prefix('=') {
            let after = after.trim_start();
            if let Some(after) = after.strip_prefix('"') {
                if let Some(end) = after.find('"') {
                    return Some(after[..end].to_string());
                }
            }
        }
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
        let names: BTreeSet<String> = extract_tools(source).into_iter().map(|t| t.name).collect();
        assert!(names.contains("cynic_health"));
        assert!(names.contains("cynic_judge"));
        assert_eq!(names.len(), 2);
    }
}
