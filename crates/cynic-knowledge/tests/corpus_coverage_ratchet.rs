use cynic_knowledge::{CORPUS_MANIFEST_SCHEMA_V1, DOCUMENT_TREE_SCHEMA_V1, scan_corpus_manifest};
use std::path::Path;

#[test]
fn complete_corpus_coverage_ratchet_holds() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();

    let manifest = scan_corpus_manifest(repo_root);

    // Schema conformance
    assert_eq!(manifest.schema, CORPUS_MANIFEST_SCHEMA_V1);
    assert!(
        !manifest.corpus_hash.is_empty(),
        "Corpus hash must be populated"
    );

    // Ratchet: 100% of tracked Markdown files must be discovered (at least 150)
    assert!(
        manifest.total_documents >= 150,
        "Expected at least 150 documents in repository, found {}",
        manifest.total_documents
    );

    assert!(
        manifest.total_sections >= 2000,
        "Expected at least 2000 sections across corpus, found {}",
        manifest.total_sections
    );

    assert!(
        manifest.total_code_blocks >= 400,
        "Expected at least 400 code blocks across corpus, found {}",
        manifest.total_code_blocks
    );

    // Verify presence of canonical pillar documents
    let paths: Vec<&str> = manifest
        .documents
        .iter()
        .map(|d| d.source_path.as_str())
        .collect();

    assert!(paths.contains(&"AGENTS.md"), "AGENTS.md must be in corpus");
    assert!(paths.contains(&"README.md"), "README.md must be in corpus");
    assert!(
        paths.contains(&"docs/foundation/CONSTITUTION.md"),
        "CONSTITUTION.md must be in corpus"
    );
    assert!(
        paths.contains(&"docs/foundation/CRATE_RULES.md"),
        "CRATE_RULES.md must be in corpus"
    );
    assert!(
        paths.contains(&"docs/CYNIC_EPISTEMIC_DATA_TRIAD.md"),
        "CYNIC_EPISTEMIC_DATA_TRIAD.md must be in corpus"
    );

    // Every document must adhere to document_tree.v1 schema and have valid properties
    for doc in &manifest.documents {
        assert_eq!(doc.schema, DOCUMENT_TREE_SCHEMA_V1);
        assert!(
            !doc.document_hash.is_empty(),
            "Doc hash must not be empty for {}",
            doc.source_path
        );
        assert!(
            doc.byte_size > 0,
            "Byte size must be > 0 for {}",
            doc.source_path
        );
        assert!(
            doc.line_count > 0,
            "Line count must be > 0 for {}",
            doc.source_path
        );
        assert!(
            !doc.root_sections.is_empty(),
            "Doc must have at least one section: {}",
            doc.source_path
        );
    }
}
