//! Executable gate for `contracts/supervisor.json`.
//!
//! The supervision contract is normative: this test fails the build when
//! the contract and the code diverge in either direction —
//! a kind named with no code witness, a witness file deleted,
//! an exit variant renamed, or a convergence fragment moved.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use cynic_protocol::supervision::SupervisorKind;
use serde_json::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate is two levels below the workspace root")
        .to_path_buf()
}

fn contract() -> Value {
    let path = workspace_root().join("contracts/supervisor.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("contracts/supervisor.json is valid JSON")
}

#[test]
fn supervisor_contract_identity_holds() {
    let contract = contract();
    assert_eq!(
        contract["schema"].as_str(),
        Some("cynic.supervisor.v1"),
        "schema must match cynic.supervisor.v1"
    );
    assert_eq!(
        contract["status"].as_str(),
        Some("normative"),
        "a normative contract gates the build; demote it explicitly or not at all"
    );
}

#[test]
fn supervisor_kinds_match_protocol_vocabulary_in_both_directions() {
    let contract = contract();
    let kinds: Vec<&str> = contract["vocabulary"]["kinds"]
        .as_array()
        .expect("vocabulary.kinds is an array")
        .iter()
        .map(|v| v.as_str().expect("kind is a string"))
        .collect();

    // Code -> contract: every L0 variant has a contract spelling.
    let all_kinds = SupervisorKind::all();
    let code_strings: BTreeSet<&str> = all_kinds.iter().map(|k| k.as_str()).collect();
    let contract_set: BTreeSet<&str> = kinds.iter().copied().collect();
    assert_eq!(
        code_strings, contract_set,
        "L0 SupervisorKind and contract kinds must name the same closed set"
    );

    // Contract -> code: every kind carries at least one on-disk witness.
    let observations = contract["vocabulary"]["kind_observations"]
        .as_array()
        .expect("vocabulary.kind_observations is an array");
    let observed_kinds: BTreeSet<&str> = observations
        .iter()
        .map(|o| o["kind"].as_str().expect("observation has a kind"))
        .collect();
    assert_eq!(
        observed_kinds, contract_set,
        "every contract kind needs a witness row, no more, no less"
    );
    for obs in observations {
        let kind = obs["kind"].as_str().unwrap();
        let witnesses = obs["witnesses"]
            .as_array()
            .unwrap_or_else(|| panic!("kind {kind} lists no witnesses"));
        assert!(
            !witnesses.is_empty(),
            "kind {kind} must name at least one witness"
        );
        for w in witnesses {
            let file = w["file"].as_str().expect("witness has a file");
            let body = fs::read_to_string(workspace_root().join(file))
                .unwrap_or_else(|e| panic!("witness file missing {file}: {e}"));
            for needle in w["contains"].as_array().expect("witness has contains") {
                let needle = needle.as_str().expect("witness needle is a string");
                assert!(
                    body.contains(needle),
                    "kind {kind}: witness {file} no longer contains {needle:?}"
                );
            }
        }
    }
}

#[test]
fn supervisor_surface_verb_exists_where_the_contract_names_it() {
    let contract = contract();
    let verb = contract["surface"]["verb"]
        .as_str()
        .expect("surface names a verb");
    let code_home = contract["surface"]["code_home"]
        .as_str()
        .expect("surface names a code home");
    // code_home is "path/To/File.rs/Enum::Arm".
    let rs_end = code_home.find(".rs").expect("code home names a .rs file") + 3;
    let file = &code_home[..rs_end];
    let arm = code_home.rsplit("::").next().expect("code home has an arm");
    let body = fs::read_to_string(workspace_root().join(file))
        .unwrap_or_else(|e| panic!("surface code home missing {file}: {e}"));
    assert!(
        body.contains(arm),
        "verb {verb}: {file} must declare the {arm} arm"
    );
    assert!(
        body.contains("observe_service_supervisor"),
        "verb {verb}: {file} must converge the host supervision observer, not invent one"
    );
    assert!(
        body.contains("desired_services()"),
        "verb {verb}: policy must come from the resolved profile, never from CLI-constructed units"
    );
}
#[test]
fn supervisor_exit_reason_lives_at_its_canonical_home() {
    let contract = contract();
    let home = contract["vocabulary"]["exit_reason"]["canonical_home"]
        .as_str()
        .expect("exit_reason names a canonical_home");
    let body = fs::read_to_string(workspace_root().join(home))
        .unwrap_or_else(|e| panic!("exit_reason home missing {home}: {e}"));
    assert!(
        body.contains("pub enum ExitReason"),
        "canonical home {home} must define pub enum ExitReason"
    );
    for variant in contract["vocabulary"]["exit_reason"]["variants"]
        .as_array()
        .expect("exit_reason lists variants")
    {
        let variant = variant.as_str().expect("variant is a string");
        assert!(
            body.contains(variant),
            "canonical home {home} no longer defines ExitReason::{variant}"
        );
    }
}

#[test]
fn supervisor_convergence_fragments_exist_on_disk() {
    let contract = contract();
    let rows = contract["convergence_map"]["rows"]
        .as_array()
        .expect("convergence_map.rows is an array");
    assert!(!rows.is_empty(), "convergence map must name its fragments");
    for row in rows {
        let fragment = row["fragment"].as_str().expect("row names a fragment");
        // A fragment names a file path followed by an optional symbol note.
        let path: String = fragment
            .chars()
            .take_while(|c| *c != ' ' && *c != '(')
            .collect();
        assert!(
            workspace_root().join(&path).exists(),
            "convergence fragment moved or deleted: {fragment}"
        );
    }
}
