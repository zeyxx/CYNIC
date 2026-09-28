use std::collections::BTreeSet;
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
    /// Every repository-relative file and directory, observed.
    ///
    /// Dot-directories are included (`.cortex`, `.hermes`, `.gitignore`) because
    /// their absence is itself a finding; `.git` is excluded as pure engine
    /// residue. This set is the observation that lets path rules answer "does
    /// this declared path exist" without any rule performing I/O.
    pub existing_paths: BTreeSet<String>,
    /// Non-comment, non-blank lines of the root `.gitignore`.
    ///
    /// Read explicitly because the walk above skips dot-files, and because a
    /// tracked authority file listed as ignored is a silent divergence between
    /// what git holds and what the working tree claims.
    pub gitignore_patterns: Vec<String>,
}

fn relative_of(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

pub fn scan_workspace(root: &Path) -> WorkspaceSourceIndex {
    let mut index = WorkspaceSourceIndex::default();
    let mut stack = vec![root.to_path_buf()];

    index.gitignore_patterns = fs::read_to_string(root.join(".gitignore"))
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

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

            if file_name == ".git" || file_name == "target" || file_name == "vendor" {
                continue;
            }

            index
                .existing_paths
                .insert(relative_of(root, &path).trim_end_matches('/').to_string());

            // Dot-directories and dot-files are observed but never traversed or
            // read. `.cargo-home/` alone holds a vendored crates.io registry
            // whose sources would otherwise contribute thousands of findings
            // about a dependency tree this repository does not own.
            if file_name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let relative_path = relative_of(root, &path);

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
