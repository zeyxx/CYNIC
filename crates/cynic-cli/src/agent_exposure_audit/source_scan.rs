use std::fs;
use std::path::Path;

use crate::rust_source::rust_files_under;

#[derive(Debug, Clone)]
pub(crate) struct McpToolDiscovery {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) line: usize,
    pub(crate) source_file: String,
}

/// Workspace-wide production scan: every observed workspace member's `src`,
/// test modules stripped (fixtures must never certify as implementations).
/// Replaces the stale single-crate scan of a `cynic-mcp` directory that does
/// not exist. An empty result is honest data — never a pass.
pub(crate) fn scan_workspace_mcp_tools() -> Vec<McpToolDiscovery> {
    let root = crate::rust_source::workspace_root();
    let prefix = format!("{}/", root.display());
    let mut tools = Vec::new();
    for crate_name in crate::rust_source::workspace_crate_names() {
        let src_root = root.join("crates").join(&crate_name).join("src");
        for mut tool in scan_mcp_tools(&src_root) {
            // scan_mcp_tools records absolute display paths; re-root them so
            // findings carry workspace-relative evidence paths.
            let relative = tool
                .source_file
                .strip_prefix(&prefix)
                .unwrap_or(&tool.source_file)
                .to_string();
            tool.source_file = relative;
            tools.push(tool);
        }
    }
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    tools
}

pub(crate) fn scan_mcp_tools(src_root: &Path) -> Vec<McpToolDiscovery> {
    let mut tools = Vec::new();
    for path in rust_files_under(src_root) {
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        // Production only: test fixtures must never certify as implementations.
        let (production, _) = crate::schema_registry_audit::split_production_and_tests(&source);
        let rel = path.display().to_string();
        tools.extend(extract_tools(production).into_iter().map(|t| {
            let line = t.line;
            McpToolDiscovery {
                source_file: rel.clone(),
                line,
                ..t
            }
        }));
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
                source_file: String::new(),
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
