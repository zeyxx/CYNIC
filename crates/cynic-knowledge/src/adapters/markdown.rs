use crate::domain::chunk::{AuthorityTier, DocumentChunk};

/// Parses a Markdown document into structural hierarchical chunks (PageIndex model).
pub fn parse_markdown_tree(source_path: &str, content: &str) -> Vec<DocumentChunk> {
    let authority = AuthorityTier::from_path(source_path);
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut current_hierarchy: Vec<String> = Vec::new();
    let mut current_lines: Vec<&str> = Vec::new();
    let mut chunk_start_line = 1;
    let mut in_code_block = false;

    for (idx, line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            current_lines.push(line);
            continue;
        }

        if !in_code_block {
            let header_level = if trimmed.starts_with("# ") {
                Some(1)
            } else if trimmed.starts_with("## ") {
                Some(2)
            } else if trimmed.starts_with("### ") {
                Some(3)
            } else if trimmed.starts_with("#### ") {
                Some(4)
            } else {
                None
            };

            if let Some(level) = header_level {
                // If we have accumulated text for a previous section, seal it into a chunk
                if !current_lines.is_empty() {
                    let section_text = current_lines.join("\n").trim().to_string();
                    if !section_text.is_empty() {
                        chunks.push(DocumentChunk::new(
                            source_path.to_string(),
                            chunk_start_line,
                            line_num - 1,
                            current_hierarchy.clone(),
                            section_text,
                            authority,
                        ));
                    }
                    current_lines.clear();
                }

                // Update hierarchy
                let title = trimmed.to_string();
                if level <= current_hierarchy.len() {
                    current_hierarchy.truncate(level - 1);
                }
                current_hierarchy.push(title);
                chunk_start_line = line_num;
            }
        }

        current_lines.push(line);
    }

    // Seal the final section
    if !current_lines.is_empty() {
        let section_text = current_lines.join("\n").trim().to_string();
        if !section_text.is_empty() {
            chunks.push(DocumentChunk::new(
                source_path.to_string(),
                chunk_start_line,
                lines.len(),
                current_hierarchy,
                section_text,
                authority,
            ));
        }
    }

    chunks
}
