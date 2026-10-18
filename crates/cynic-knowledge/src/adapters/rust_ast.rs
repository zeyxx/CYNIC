use crate::domain::chunk::{AuthorityTier, DocumentChunk};
use crate::domain::chunk_quality::{MAX_CHUNK_CHARS, split_span_by_char_budget};

/// Parses a Rust source file and extracts semantic AST chunks (module docstrings,
/// public structs, enums, functions, traits, and implementations).
pub fn parse_rust_file(rel_path: &str, content: &str) -> Vec<DocumentChunk> {
    let mut chunks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return chunks;
    }

    let authority = derive_rust_authority(rel_path);
    let hierarchy_prefix = derive_hierarchy_prefix(rel_path);

    let mut idx = 0;

    // 1. Module-level docstrings (//! ...)
    let mut module_docs = Vec::new();
    let module_doc_start = 1;
    while idx < lines.len() && lines[idx].trim_start().starts_with("//!") {
        module_docs.push(lines[idx]);
        idx += 1;
    }
    if !module_docs.is_empty() {
        let hierarchy = {
            let mut h = hierarchy_prefix.clone();
            h.push("Module Overview".to_string());
            h
        };
        // Char-budget guard: long module headers (policy preambles) must also
        // fit the dense window, so route them through the same splitter.
        let joined = module_docs.join("\n");
        if joined.len() > MAX_CHUNK_CHARS {
            for sub in split_span_by_char_budget(
                rel_path,
                module_doc_start,
                idx,
                &hierarchy_prefix,
                "Module Overview",
                &lines[module_doc_start - 1..idx],
                authority,
            ) {
                chunks.push(sub);
            }
        } else {
            chunks.push(DocumentChunk::new(
                rel_path.to_string(),
                module_doc_start,
                idx,
                hierarchy,
                joined,
                authority,
            ));
        }
    }

    // 2. Scan declarations and implementations
    while idx < lines.len() {
        let line = lines[idx];
        let trimmed = line.trim();

        // Capture docstrings preceding an item
        if trimmed.starts_with("///") {
            let doc_start = idx + 1;
            let mut doc_lines = vec![line];
            idx += 1;
            while idx < lines.len() && lines[idx].trim().starts_with("///") {
                doc_lines.push(lines[idx]);
                idx += 1;
            }

            // Next non-doc line could be an attribute (#[...]) or declaration
            while idx < lines.len() && lines[idx].trim().starts_with("#[") {
                doc_lines.push(lines[idx]);
                idx += 1;
            }

            if idx < lines.len() {
                let decl_line = lines[idx].trim();
                if is_declaration_start(decl_line) {
                    let item_start = doc_start;
                    let (item_end, body_lines) = capture_item_body(&lines, idx);
                    doc_lines.extend(body_lines);
                    let signature = extract_signature(decl_line);
                    emit_rust_chunks(
                        rel_path,
                        item_start,
                        item_end,
                        &hierarchy_prefix,
                        &signature,
                        doc_lines.join("\n"),
                        authority,
                        &lines,
                        &mut chunks,
                    );
                    idx = item_end;
                    continue;
                }
            }
        } else if is_declaration_start(trimmed) {
            let item_start = idx + 1;
            let (item_end, body_lines) = capture_item_body(&lines, idx);
            let signature = extract_signature(trimmed);
            emit_rust_chunks(
                rel_path,
                item_start,
                item_end,
                &hierarchy_prefix,
                &signature,
                body_lines.join("\n"),
                authority,
                &lines,
                &mut chunks,
            );
            idx = item_end;
            continue;
        }

        idx += 1;
    }

    chunks
}

const MAX_AST_CHUNK_LINES: usize = 70;
const AST_WINDOW_SIZE: usize = 50;
const AST_WINDOW_OVERLAP: usize = 10;
const MIN_CONTEXT_CHARS: usize = 60;

#[allow(clippy::too_many_arguments)]
fn emit_rust_chunks(
    rel_path: &str,
    item_start: usize,
    item_end: usize,
    hierarchy_prefix: &[String],
    signature: &str,
    content: String,
    authority: AuthorityTier,
    file_lines: &[&str],
    chunks: &mut Vec<DocumentChunk>,
) {
    let line_count = item_end.saturating_sub(item_start) + 1;
    let trimmed_len = content.trim().len();

    // 1. Starved micro-item enrichment
    let final_content = if trimmed_len < MIN_CONTEXT_CHARS {
        format!("// Context: {} ({})\n{}", rel_path, signature, content)
    } else {
        content
    };

    // 2. Stage 1: line-bounded windowed decomposition for long items.
    //     Stage 2 (char budget) handles windows whose lines are individually long.
    if line_count > MAX_AST_CHUNK_LINES && file_lines.len() >= item_end {
        let mut curr_start = item_start;
        while curr_start <= item_end {
            let curr_end = (curr_start + AST_WINDOW_SIZE - 1).min(item_end);
            let mut window_content = file_lines[curr_start - 1..curr_end].join("\n");
            if window_content.trim().len() < MIN_CONTEXT_CHARS {
                window_content = format!(
                    "// Context: {} ({} lines {}-{})\n{}",
                    rel_path, signature, curr_start, curr_end, window_content
                );
            }

            // Stage 2 guard: long lines can push a 50-line window past the char budget.
            // No enrichment happened here (content is far above MIN_CONTEXT_CHARS),
            // so the file slice and window_content are byte-identical.
            if window_content.len() > MAX_CHUNK_CHARS {
                let window_slice = &file_lines[curr_start - 1..curr_end];
                for sub in split_span_by_char_budget(
                    rel_path,
                    curr_start,
                    curr_end,
                    hierarchy_prefix,
                    signature,
                    window_slice,
                    authority,
                ) {
                    chunks.push(sub);
                }
            } else {
                let mut hierarchy = hierarchy_prefix.to_vec();
                hierarchy.push(format!("{} (lines {}-{})", signature, curr_start, curr_end));
                chunks.push(DocumentChunk::new(
                    rel_path.to_string(),
                    curr_start,
                    curr_end,
                    hierarchy,
                    window_content,
                    authority,
                ));
            }

            if curr_end >= item_end {
                break;
            }
            curr_start += AST_WINDOW_SIZE - AST_WINDOW_OVERLAP;
        }
    } else if final_content.len() > MAX_CHUNK_CHARS && file_lines.len() >= item_end {
        // Single short-line-count item whose lines are individually huge.
        for sub in split_span_by_char_budget(
            rel_path,
            item_start,
            item_end,
            hierarchy_prefix,
            signature,
            file_lines,
            authority,
        ) {
            chunks.push(sub);
        }
    } else {
        let mut hierarchy = hierarchy_prefix.to_vec();
        hierarchy.push(signature.to_string());
        chunks.push(DocumentChunk::new(
            rel_path.to_string(),
            item_start,
            item_end,
            hierarchy,
            final_content,
            authority,
        ));
    }
}

/// Checks if a line initiates an extractable Rust declaration.
fn is_declaration_start(line: &str) -> bool {
    line.starts_with("pub struct ")
        || line.starts_with("pub enum ")
        || line.starts_with("pub trait ")
        || line.starts_with("pub fn ")
        || line.starts_with("pub async fn ")
        || line.starts_with("pub type ")
        || line.starts_with("pub const ")
        || line.starts_with("impl ")
        || line.starts_with("impl<")
}

/// Captures the full body of a declaration by balancing curly braces.
fn capture_item_body<'a>(lines: &'a [&'a str], start_idx: usize) -> (usize, Vec<&'a str>) {
    let mut collected = Vec::new();
    let mut brace_depth: i32 = 0;
    let mut found_open = false;
    let mut curr = start_idx;

    while curr < lines.len() {
        let line = lines[curr];
        collected.push(line);

        for ch in line.chars() {
            if ch == '{' {
                brace_depth += 1;
                found_open = true;
            } else if ch == '}' {
                brace_depth -= 1;
            }
        }

        // If statement terminates with semicolon without opening braces (e.g. `pub type X = Y;`)
        if !found_open && line.trim().ends_with(';') {
            return (curr + 1, collected);
        }

        // If braces were opened and now closed back to zero
        if found_open && brace_depth <= 0 {
            return (curr + 1, collected);
        }

        curr += 1;
    }

    (curr, collected)
}

/// Shortens declaration line to a clean hierarchical title.
fn extract_signature(line: &str) -> String {
    let s = line.trim();
    if let Some(pos) = s.find('{') {
        s[..pos].trim().to_string()
    } else if let Some(pos) = s.find(';') {
        s[..pos].trim().to_string()
    } else {
        s.to_string()
    }
}

/// Derives hierarchy prefix [crate, filename] from relative path.
fn derive_hierarchy_prefix(rel_path: &str) -> Vec<String> {
    let parts: Vec<&str> = rel_path.split('/').collect();
    if parts.len() >= 2 && parts[0] == "crates" {
        let crate_name = parts[1];
        let file_stem = parts.last().unwrap_or(&"file").trim_end_matches(".rs");
        vec![crate_name.to_string(), file_stem.to_string()]
    } else {
        vec!["src".to_string(), rel_path.to_string()]
    }
}

/// Derives authority tier from crate topology and layer conventions.
fn derive_rust_authority(rel_path: &str) -> AuthorityTier {
    if rel_path.contains("cynic-types")
        || rel_path.contains("cynic-identity")
        || rel_path.contains("cynic-protocol")
    {
        AuthorityTier::L0Constitution
    } else if rel_path.contains("cynic-kernel")
        || rel_path.contains("cynic-inference")
        || rel_path.contains("cynic-host")
    {
        AuthorityTier::L1Specification
    } else if rel_path.contains("cynic-knowledge")
        || rel_path.contains("cynic-discovery")
        || rel_path.contains("cynic-storage")
    {
        AuthorityTier::L2Design
    } else {
        AuthorityTier::L3Operation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust_file_extracts_items_with_docstrings() {
        let code = r#"//! Module level documentation.
//! Second line of module docs.

use std::path::PathBuf;

/// Path resolution utility.
#[derive(Debug)]
pub struct CynicPaths {
    pub root: PathBuf,
}

impl CynicPaths {
    /// Constructs a new instance.
    pub fn new() -> Self {
        Self { root: PathBuf::from("/") }
    }
}

pub fn standalone_function() -> u32 {
    42
}
"#;

        let chunks = parse_rust_file("crates/cynic-types/src/paths.rs", code);
        assert!(!chunks.is_empty());

        // 1. Module overview
        assert_eq!(chunks[0].section_hierarchy[2], "Module Overview");
        assert_eq!(chunks[0].authority, AuthorityTier::L0Constitution);

        // 2. Struct
        let struct_chunk = chunks
            .iter()
            .find(|c| c.content.contains("pub struct CynicPaths"))
            .expect("struct chunk");
        assert!(
            struct_chunk
                .content
                .contains("/// Path resolution utility.")
        );
        assert!(struct_chunk.content.contains("pub root: PathBuf"));

        // 3. Impl
        let impl_chunk = chunks
            .iter()
            .find(|c| c.content.contains("impl CynicPaths"))
            .expect("impl chunk");
        assert!(impl_chunk.content.contains("pub fn new() -> Self"));

        // 4. Function
        let fn_chunk = chunks
            .iter()
            .find(|c| c.content.contains("pub fn standalone_function"))
            .expect("fn chunk");
        assert!(fn_chunk.content.contains("42"));
    }
}
