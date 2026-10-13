use cynic_knowledge::{GIT_COMMIT_SCHEMA_V1, extract_git_history, parse_git_log_output};
use std::path::Path;

#[test]
fn pure_git_log_parser_reconstructs_records() {
    let raw = r#"
COMMIT_START
395dfd63a83473fb6be644af466ddbaae3fd2a69
e7963b7276f1986ef1721652a864f360fc314960
hermes
hermes@agent.local
1791852996
chore(vercel): disable broken auto-deploy on Rust-only repo

Detailed explanation here.
COMMIT_FILES
vercel.json
crates/cynic-knowledge/Cargo.toml
"#;

    let records = parse_git_log_output(raw);
    assert_eq!(records.len(), 1);
    let r = &records[0];
    assert_eq!(r.schema, GIT_COMMIT_SCHEMA_V1);
    assert_eq!(r.hash, "395dfd63a83473fb6be644af466ddbaae3fd2a69");
    assert_eq!(
        r.parent_hashes,
        vec!["e7963b7276f1986ef1721652a864f360fc314960"]
    );
    assert_eq!(r.author_name, "hermes");
    assert_eq!(r.author_email, "hermes@agent.local");
    assert_eq!(r.timestamp, 1791852996);
    assert_eq!(
        r.subject,
        "chore(vercel): disable broken auto-deploy on Rust-only repo"
    );
    assert_eq!(r.body, "Detailed explanation here.");
    assert_eq!(
        r.files_changed,
        vec!["vercel.json", "crates/cynic-knowledge/Cargo.toml"]
    );

    let chunk = r.to_chunk();
    assert!(chunk.source_path.starts_with("git://commit/"));
    assert!(chunk.content.contains("vercel.json"));
}

#[test]
fn workspace_git_history_extraction_holds() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();

    let commits = extract_git_history(repo_root, 15).expect("Should extract git commits");
    assert!(!commits.is_empty(), "Git commits must not be empty");
    assert!(commits.len() >= 5, "Expected at least 5 recent commits");

    for c in &commits {
        assert!(!c.hash.is_empty());
        assert!(!c.author_name.is_empty());
        assert!(c.timestamp > 0);
        assert!(!c.subject.is_empty());
    }
}
