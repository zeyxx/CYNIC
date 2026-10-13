use cynic_knowledge::{
    AuthorityTier, DocumentChunk, SYNTHESIS_PROMPT_SCHEMA_V1, SearchHit, SynthesisPromptBuilder,
};

#[test]
fn synthesis_prompt_builder_enforces_epistemic_grounding() {
    let chunk1 = DocumentChunk::new(
        "AGENTS.md".to_string(),
        20,
        26,
        vec!["Invariants".to_string()],
        "Bottom-Up over Top-Down invariant holds unconditionally.".to_string(),
        AuthorityTier::L0Constitution,
    );
    let chunk2 = DocumentChunk::new(
        "docs/specs/checklist.md".to_string(),
        45,
        50,
        vec!["Hardware".to_string()],
        "RAM cortex has 2487 errors at XMP 3200.".to_string(),
        AuthorityTier::L1Specification,
    );

    let hits = vec![
        SearchHit {
            chunk: &chunk1,
            score: 18.5,
        },
        SearchHit {
            chunk: &chunk2,
            score: 12.0,
        },
    ];

    let prompt = SynthesisPromptBuilder::build("Quels sont les invariants matériels ?", &hits);

    assert_eq!(prompt.schema, SYNTHESIS_PROMPT_SCHEMA_V1);
    assert_eq!(prompt.sources.len(), 2);
    assert!(prompt.system_instruction.contains("VÉRITÉ ANCRÉE"));
    assert!(
        prompt
            .system_instruction
            .contains("NON COUVERT PAR LE CORPUS ACTUEL")
    );
    assert!(
        prompt
            .user_prompt
            .contains("[SOURCE 1] AGENTS.md (Lignes 20-26")
    );
    assert!(
        prompt
            .user_prompt
            .contains("[SOURCE 2] docs/specs/checklist.md")
    );
    assert!(prompt.user_prompt.contains("QUESTION DE L'OPÉRATEUR :"));
}
