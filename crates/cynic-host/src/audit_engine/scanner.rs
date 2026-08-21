use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub line_count: usize,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceSourceIndex {
    pub rust_files: Vec<SourceFile>,
    pub json_files: Vec<SourceFile>,
    pub manifests: Vec<SourceFile>,
}

pub fn scan_workspace(root: &Path) -> WorkspaceSourceIndex {
    let mut index = WorkspaceSourceIndex::default();
    let mut stack = vec![root.to_path_buf()];

    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();

            if file_name.starts_with('.') || file_name == "target" || file_name == "vendor" {
                continue;
            }

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace('\\', "/");

            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };

            let line_count = content.lines().count();
            let source = SourceFile {
                relative_path,
                absolute_path: path.clone(),
                line_count,
                content,
            };

            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                index.rust_files.push(source);
            } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                index.json_files.push(source);
            } else if file_name == "Cargo.toml" {
                index.manifests.push(source);
            }
        }
    }

    index
        .rust_files
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    index
        .json_files
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    index
        .manifests
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    index
}
