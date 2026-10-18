use crate::domain::ast::{
    CodeBlock, DocumentTree, MarkdownLink, SectionNode, DOCUMENT_TREE_SCHEMA_V1,
};
use crate::domain::chunk::{AuthorityTier, DocumentChunk};
use crate::domain::chunk_quality::{split_span_by_char_budget, MAX_CHUNK_CHARS};
use regex::Regex;
use std::sync::LazyLock;

static LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap());

/// Parses a Markdown document into a complete hierarchical AST tree (PageIndex model).
pub fn parse_document_tree(source_path: &str, content: &str) -> DocumentTree {
    let authority = AuthorityTier::from_path(source_path);
    let lines: Vec<&str> = content.lines().collect();
    let line_count = lines.len();
    let byte_size = content.len();
    let document_hash = blake3::hash(content.as_bytes()).to_hex().to_string();

    let mut doc_title = source_path.to_string();
    let mut flat_sections: Vec<SectionBuilder> = Vec::new();

    let mut in_code_block = false;
    let mut current_code_lang: Option<String> = None;
    let mut current_code_lines: Vec<String> = Vec::new();
    let mut code_start_line = 0;

    let mut current_section_lines: Vec<String> = Vec::new();
    let mut current_section_code_blocks: Vec<CodeBlock> = Vec::new();
    let mut current_section_links: Vec<MarkdownLink> = Vec::new();
    let mut current_title = "Preamble".to_string();
    let mut current_level = 1;
    let mut section_start_line = 1;

    for (idx, line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                // End of code block
                current_section_code_blocks.push(CodeBlock {
                    language: current_code_lang.take(),
                    code: current_code_lines.join("\n"),
                    line_start: code_start_line,
                    line_end: line_num,
                });
                current_code_lines.clear();
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
                code_start_line = line_num;
                let lang_str = trimmed.trim_start_matches("```").trim();
                current_code_lang = if lang_str.is_empty() {
                    None
                } else {
                    Some(lang_str.to_string())
                };
            }
            current_section_lines.push(line.to_string());
            continue;
        }

        if in_code_block {
            current_code_lines.push(line.to_string());
            current_section_lines.push(line.to_string());
            continue;
        }

        // Header detection outside code blocks
        let header_info = if trimmed.starts_with("# ") {
            Some((1, trimmed.trim_start_matches("# ").trim()))
        } else if trimmed.starts_with("## ") {
            Some((2, trimmed.trim_start_matches("## ").trim()))
        } else if trimmed.starts_with("### ") {
            Some((3, trimmed.trim_start_matches("### ").trim()))
        } else if trimmed.starts_with("#### ") {
            Some((4, trimmed.trim_start_matches("#### ").trim()))
        } else {
            None
        };

        if let Some((level, h_title)) = header_info {
            // First H1 can serve as document title
            if level == 1 && doc_title == source_path {
                doc_title = h_title.to_string();
            }

            // Flush previous section if it has content
            if !current_section_lines.is_empty() {
                let sec_content = current_section_lines.join("\n").trim().to_string();
                if !sec_content.is_empty() {
                    flat_sections.push(SectionBuilder {
                        level: current_level,
                        title: current_title.clone(),
                        line_start: section_start_line,
                        line_end: line_num - 1,
                        content: sec_content,
                        code_blocks: std::mem::take(&mut current_section_code_blocks),
                        links: std::mem::take(&mut current_section_links),
                    });
                }
                current_section_lines.clear();
            }

            current_title = h_title.to_string();
            current_level = level;
            section_start_line = line_num;
        }

        // Extract markdown links from text
        for cap in LINK_REGEX.captures_iter(line) {
            if let (Some(label), Some(target)) = (cap.get(1), cap.get(2)) {
                current_section_links.push(MarkdownLink {
                    label: label.as_str().to_string(),
                    target: target.as_str().to_string(),
                });
            }
        }

        current_section_lines.push(line.to_string());
    }

    // Flush final section
    if !current_section_lines.is_empty() {
        let sec_content = current_section_lines.join("\n").trim().to_string();
        if !sec_content.is_empty() {
            flat_sections.push(SectionBuilder {
                level: current_level,
                title: current_title,
                line_start: section_start_line,
                line_end: line_count,
                content: sec_content,
                code_blocks: current_section_code_blocks,
                links: current_section_links,
            });
        }
    }

    // Convert flat builders into hierarchical recursive SectionNode tree
    let root_sections = build_hierarchy(source_path, flat_sections);

    DocumentTree {
        schema: DOCUMENT_TREE_SCHEMA_V1.to_string(),
        source_path: source_path.to_string(),
        document_hash,
        byte_size,
        line_count,
        authority,
        title: doc_title,
        root_sections,
    }
}

struct SectionBuilder {
    level: usize,
    title: String,
    line_start: usize,
    line_end: usize,
    content: String,
    code_blocks: Vec<CodeBlock>,
    links: Vec<MarkdownLink>,
}

fn build_hierarchy(source_path: &str, builders: Vec<SectionBuilder>) -> Vec<SectionNode> {
    if builders.is_empty() {
        return Vec::new();
    }

    let mut roots: Vec<SectionNode> = Vec::new();
    let mut stack: Vec<SectionNode> = Vec::new();

    for b in builders {
        let hash_input = format!("{}:{}:{}", source_path, b.line_start, b.content);
        let section_id = blake3::hash(hash_input.as_bytes()).to_hex().to_string();

        let node = SectionNode {
            section_id,
            level: b.level,
            title: b.title,
            line_start: b.line_start,
            line_end: b.line_end,
            content: b.content,
            code_blocks: b.code_blocks,
            links: b.links,
            children: Vec::new(),
        };

        while let Some(top) = stack.last() {
            if top.level >= node.level {
                let finished = stack.pop().unwrap();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(finished);
                } else {
                    roots.push(finished);
                }
            } else {
                break;
            }
        }

        stack.push(node);
    }

    while let Some(finished) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(finished);
        } else {
            roots.push(finished);
        }
    }

    roots
}

/// Parses a Markdown document into structural hierarchical chunks (flat view for BM25).
pub fn parse_markdown_tree(source_path: &str, content: &str) -> Vec<DocumentChunk> {
    let tree = parse_document_tree(source_path, content);
    let mut chunks = Vec::new();
    flatten_tree_chunks(
        &tree.source_path,
        tree.authority,
        &tree.root_sections,
        &mut Vec::new(),
        &mut chunks,
    );
    chunks
}

fn flatten_tree_chunks(
    source_path: &str,
    authority: AuthorityTier,
    nodes: &[SectionNode],
    hierarchy: &mut Vec<String>,
    chunks: &mut Vec<DocumentChunk>,
) {
    for node in nodes {
        hierarchy.push(format!("{} {}", "#".repeat(node.level), node.title));
        let label = hierarchy.last().cloned().unwrap_or_default();

        // Char-budget guard: a markdown section with individually long lines
        // (compact tables, dense reference lists) can blow past the dense
        // model's window even with a modest line count.
        if node.content.len() > MAX_CHUNK_CHARS {
            let lines: Vec<&str> = node.content.split('\n').collect();
            let mut emitted = 0usize;
            for sub in split_span_by_char_budget(
                source_path,
                node.line_start,
                node.line_end,
                hierarchy,
                &label,
                &lines,
                authority,
            ) {
                // Re-map sub-window offsets onto physical file line coordinates.
                let span = sub.line_end - sub.line_start + 1;
                let physical_start = node.line_start + emitted;
                let physical_end = (physical_start + span - 1).min(node.line_end);
                emitted += span;
                chunks.push(DocumentChunk::new(
                    source_path.to_string(),
                    physical_start,
                    physical_end,
                    sub.section_hierarchy,
                    sub.content,
                    authority,
                ));
            }
        } else {
            chunks.push(DocumentChunk::new(
                source_path.to_string(),
                node.line_start,
                node.line_end,
                hierarchy.clone(),
                node.content.clone(),
                authority,
            ));
        }
        flatten_tree_chunks(source_path, authority, &node.children, hierarchy, chunks);
        hierarchy.pop();
    }
}
