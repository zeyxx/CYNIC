use cynic_knowledge::{AuthorityTier, index_workspace, parse_markdown_tree};
use std::path::Path;

#[test]
fn markdown_parser_preserves_code_blocks_and_hierarchy() {
    let sample = r#"# Architecture Overview

This is an intro paragraph.

## Section 1: Invariants

Here is some text.

```rust
fn do_not_split_this() {
    let x = 42;
    // # This is not a header!
}
```

## Section 2: Closing

Final thoughts.
"#;

    let chunks = parse_markdown_tree("docs/sample.md", sample);
    assert_eq!(chunks.len(), 3);

    // Section 1 must contain the entire intact code block
    let sec1 = &chunks[1];
    assert!(
        sec1.section_hierarchy
            .contains(&"## Section 1: Invariants".to_string())
    );
    assert!(sec1.content.contains("fn do_not_split_this()"));
    assert!(sec1.content.contains("// # This is not a header!"));
}

#[test]
fn authority_tiers_are_correctly_classified() {
    assert_eq!(
        AuthorityTier::from_path("contracts/repository-governance.json"),
        AuthorityTier::L0Constitution
    );
    assert_eq!(
        AuthorityTier::from_path("AGENTS.md"),
        AuthorityTier::L0Constitution
    );
    assert_eq!(
        AuthorityTier::from_path("docs/foundation/CONSTITUTION.md"),
        AuthorityTier::L0Constitution
    );
    assert_eq!(
        AuthorityTier::from_path("docs/specs/my-spec.md"),
        AuthorityTier::L1Specification
    );
    assert_eq!(
        AuthorityTier::from_path("docs/design/my-design.md"),
        AuthorityTier::L2Design
    );
}

#[test]
fn workspace_indexing_and_bm25_search_works() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();

    let index = index_workspace(repo_root);
    assert!(
        index.chunk_count() > 100,
        "Workspace must have over 100 chunks"
    );

    // Search for a constitutional invariant
    let hits = index.search("Bottom-Up over Top-Down", 5);
    assert!(!hits.is_empty(), "Must find Bottom-Up invariant");
    assert!(
        hits[0].chunk.content.contains("Bottom-Up")
            || hits[0].chunk.content.contains("physical reality")
    );

    // Search for state leak blacklist
    let hits_blacklist = index.search("state_leak_blacklist", 5);
    assert!(!hits_blacklist.is_empty(), "Must find state_leak_blacklist");
    assert!(
        hits_blacklist[0]
            .chunk
            .source_path
            .contains("repository-governance.json")
            || hits_blacklist[0]
                .chunk
                .content
                .contains("state_leak_blacklist")
    );
}
