use std::collections::{BTreeMap, BTreeSet};
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
    /// What the live system says, observed once so no rule performs I/O.
    ///
    /// Empty on any host that is not the declared node: a rule that read the
    /// host itself would be a test that lies everywhere else, and an audit that
    /// silently skips its own domain is how the domain stops being checked.
    /// `None` means "not observed here", which is an explicit unknown rather
    /// than a pass.
    pub host: Option<HostObservation>,
}

/// The four divergences that only a live system can settle, read once.
#[derive(Debug, Clone, Default)]
pub struct HostObservation {
    /// systemd unit files present on the host, absolute paths.
    pub unit_files: Vec<PathBuf>,
    /// Unit file text, keyed by the unit file's path.
    ///
    /// Held so a rule can report the FILE and the VARIABLE NAME of a credential
    /// without ever putting a value into a report, which is the whole point of
    /// reading it here instead of printing it.
    pub unit_contents: BTreeMap<PathBuf, String>,
    /// Resolved ExecStart targets of the units, with the unit that declared it.
    pub exec_starts: Vec<(String, PathBuf)>,
    /// The claim store, parsed. Kept as text because a host is not a Rust
    /// project and this module must not depend on the coordination crate.
    pub claim_store: Option<String>,
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

    index.host = observe_host();

    index
}

/// Observe the live system. `None` when this is not a host with systemd.
///
/// A unit directory that exists but cannot be read is reported as `None` too:
/// an audit that cannot see the host must not claim the host is clean.
fn observe_host() -> Option<HostObservation> {
    // The unit directories are overridable so this rule can be proven against a
    // fixture instead of only against whichever machine happens to run it. A
    // security check that can only be exercised by breaking the real host is a
    // check nobody runs.
    let unit_dirs: Vec<String> = match std::env::var("CYNIC_AUDIT_UNIT_DIRS") {
        Ok(value) if !value.trim().is_empty() => value
            .split(':')
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(str::to_string)
            .collect(),
        _ => vec![
            "/etc/systemd/system".to_string(),
            "/usr/lib/systemd/system".to_string(),
        ],
    };
    let mut observation = HostObservation::default();
    let mut found_any = false;

    for dir in unit_dirs {
        let dir = PathBuf::from(dir);
        let dir = dir.as_path();
        if !dir.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return None;
        };
        found_any = true;
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !(name.ends_with(".service") || name.ends_with(".timer")) {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            for exec in text.lines() {
                let Some(rest) = exec.trim().strip_prefix("ExecStart=") else {
                    continue;
                };
                let mut words = rest
                    .split_whitespace()
                    .filter(|word| !word.starts_with('-') && *word != "sudo");
                if let Some(program) = words.next() {
                    if program.starts_with('/') {
                        observation.exec_starts.push((name.to_string(), PathBuf::from(program)));
                    }
                }
            }
            observation.unit_contents.insert(path.clone(), text);
            observation.unit_files.push(path);
        }
    }

    if !found_any {
        return None;
    }

    observation.unit_files.sort();
    observation.exec_starts.sort();
    observation.claim_store = fs::read_to_string("/var/lib/cynic/.agent-claims.json")
        .ok()
        .or_else(|| fs::read_to_string(".agent-claims.json").ok());

    Some(observation)
}
