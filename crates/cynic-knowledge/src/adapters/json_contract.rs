use crate::domain::authority::{AuthorityOverrides, AUTHORITY_TIERS_REGISTRY_PATH};
use crate::domain::chunk::{AuthorityTier, DocumentChunk};
use crate::domain::chunk_quality::{split_span_by_char_budget, MAX_CHUNK_CHARS};
use serde_json::Value;
use std::path::Path;

/// Loads the authority-tier registry for a workspace root.
/// Missing or malformed registry fails open to empty overrides (directory
/// defaults apply); tiers are ranking hints, never a security gate.
pub fn load_authority_overrides<P: AsRef<Path>>(root: P) -> AuthorityOverrides {
    let path = root.as_ref().join(AUTHORITY_TIERS_REGISTRY_PATH);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| AuthorityOverrides::from_json(&text).ok())
        .unwrap_or_default()
}

/// Parses a JSON contract into structural chunks with directory-default authority.
pub fn parse_json_contract(source_path: &str, content: &str) -> Vec<DocumentChunk> {
    parse_json_contract_with_authority(source_path, content, AuthorityTier::from_path(source_path))
}

/// Parses a JSON contract into structural chunks with caller-resolved authority.
pub fn parse_json_contract_with_authority(
    source_path: &str,
    content: &str,
    authority: AuthorityTier,
) -> Vec<DocumentChunk> {
    let mut chunks = Vec::new();
    let parsed: Result<Value, _> = serde_json::from_str(content);

    let val = match parsed {
        Ok(v) => v,
        Err(_) => {
            // Fallback: whole raw JSON as single chunk
            return vec![DocumentChunk::new(
                source_path.to_string(),
                1,
                content.lines().count().max(1),
                vec![format!("Contract: {}", source_path)],
                content.to_string(),
                authority,
            )];
        }
    };

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len().max(1);

    if let Value::Object(map) = val {
        let title = map
            .get("name")
            .or_else(|| map.get("contract"))
            .or_else(|| map.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or(source_path)
            .to_string();

        let spans = find_json_top_level_spans(content);

        // Group into concise chunks with exact line coordinates
        for (key, section_val) in map {
            let (line_start, line_end, chunk_content) = if let Some(&(start, end)) = spans.get(&key)
            {
                let slice = lines[start.saturating_sub(1)..end.min(total_lines)].join("\n");
                (start, end, slice)
            } else {
                let section_str = serde_json::to_string_pretty(&section_val).unwrap_or_default();
                (1, total_lines, format!("{}:\n{}", key, section_str))
            };

            let line_count = line_end.saturating_sub(line_start) + 1;
            if line_count > 70 && line_end <= total_lines {
                let mut curr_start = line_start;
                while curr_start <= line_end {
                    let curr_end = (curr_start + 50 - 1).min(line_end);
                    let window_slice = lines[curr_start - 1..curr_end].join("\n");
                    let window_refs: Vec<&str> = lines[curr_start - 1..curr_end].to_vec();
                    let prefix = vec![title.clone()];

                    if window_slice.len() > MAX_CHUNK_CHARS {
                        for sub in split_span_by_char_budget(
                            source_path,
                            curr_start,
                            curr_end,
                            &prefix,
                            &key,
                            &window_refs,
                            authority,
                        ) {
                            chunks.push(sub);
                        }
                    } else {
                        chunks.push(DocumentChunk::new(
                            source_path.to_string(),
                            curr_start,
                            curr_end,
                            vec![
                                title.clone(),
                                format!("{key} (lines {curr_start}-{curr_end})"),
                            ],
                            window_slice,
                            authority,
                        ));
                    }
                    if curr_end >= line_end {
                        break;
                    }
                    curr_start += 50 - 10;
                }
            } else if chunk_content.len() > MAX_CHUNK_CHARS && line_end <= total_lines {
                let window_refs: Vec<&str> = lines[line_start - 1..line_end].to_vec();
                for sub in split_span_by_char_budget(
                    source_path,
                    line_start,
                    line_end,
                    &[title.clone()],
                    &key,
                    &window_refs,
                    authority,
                ) {
                    chunks.push(sub);
                }
            } else {
                chunks.push(DocumentChunk::new(
                    source_path.to_string(),
                    line_start,
                    line_end,
                    vec![title.clone(), key],
                    chunk_content,
                    authority,
                ));
            }
        }
    } else {
        chunks.push(DocumentChunk::new(
            source_path.to_string(),
            1,
            total_lines,
            vec![format!("Contract: {}", source_path)],
            content.to_string(),
            AuthorityTier::L0Constitution,
        ));
    }

    chunks
}

/// Locates 1-indexed (line_start, line_end) for top-level JSON keys in formatted text.
fn find_json_top_level_spans(content: &str) -> std::collections::HashMap<String, (usize, usize)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut spans = std::collections::HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_start = 1;

    for (idx, line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim_start();
        if (line.starts_with("  \"") || line.starts_with("\t\"")) && trimmed.starts_with('"') {
            if let Some(colon_pos) = trimmed.find("\":") {
                let key = &trimmed[1..colon_pos];
                if let Some(prev_key) = current_key.take() {
                    let end_line = line_num.saturating_sub(1);
                    spans.insert(prev_key, (current_start, end_line.max(current_start)));
                }
                current_key = Some(key.to_string());
                current_start = line_num;
            }
        }
    }

    if let Some(prev_key) = current_key {
        let mut end_line = lines.len();
        while end_line > current_start
            && (lines[end_line - 1].trim() == "}" || lines[end_line - 1].trim().is_empty())
        {
            end_line -= 1;
        }
        spans.insert(prev_key, (current_start, end_line.max(current_start)));
    }

    spans
}
