use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn find_duplicate_trait_names(project_root: &Path) -> Vec<String> {
    let domain_path = project_root.join("crates/cynic-kernel/src/domain");
    if !domain_path.is_dir() {
        return Vec::new();
    }
    duplicate_trait_names_in_dir(&domain_path)
}

pub fn duplicate_trait_names_in_dir(domain_path: &Path) -> Vec<String> {
    let trait_regex = regex::Regex::new(r"^\s*(?:pub(?:\([^)]+\))?\s+)?trait\s+(\w+)")
        .expect("trait regex must compile");
    let mut counts = BTreeMap::<String, usize>::new();

    for file_path in walk_rust_files(domain_path) {
        let Ok(content) = fs::read_to_string(&file_path) else {
            continue;
        };
        for capture in trait_regex.captures_iter(&content) {
            if let Some(name) = capture.get(1) {
                *counts.entry(name.as_str().to_string()).or_default() += 1;
            }
        }
    }

    counts
        .into_iter()
        .filter_map(|(name, count)| (count > 1).then_some(name))
        .collect()
}

fn walk_rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_rust_files(&path));
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_duplicate_trait_names() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-inference-duplicate-traits-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(temp.join("nested")).expect("fixture directory should be created");
        fs::write(temp.join("a.rs"), "pub trait Alpha {}\n")
            .expect("fixture file should be written");
        fs::write(temp.join("nested/b.rs"), "trait Alpha {}\ntrait Beta {}\n")
            .expect("fixture file should be written");

        let duplicates = duplicate_trait_names_in_dir(&temp);
        let _ = fs::remove_dir_all(&temp);

        assert_eq!(duplicates, vec!["Alpha".to_string()]);
    }

    #[test]
    fn ignores_unique_trait_names() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-inference-duplicate-traits-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp).expect("fixture directory should be created");
        fs::write(temp.join("a.rs"), "pub trait Alpha {}\ntrait Beta {}\n")
            .expect("fixture file should be written");

        let duplicates = duplicate_trait_names_in_dir(&temp);
        let _ = fs::remove_dir_all(&temp);

        assert!(duplicates.is_empty());
    }
}
