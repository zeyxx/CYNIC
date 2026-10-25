use cynic_knowledge::{
    check_freshness, epoch_seconds, git_proof_resolver, parse_proof_reference, verify_event_proofs, HistoricalEvent,
    ProofOutcome,
};
use std::path::Path;

fn event(title: &str, proof: Option<&str>) -> HistoricalEvent {
    HistoricalEvent {
        date: "2026-01-01".to_string(),
        title: title.to_string(),
        description_fr: String::new(),
        description_en: String::new(),
        proof_reference: proof.map(str::to_string),
    }
}

#[test]
fn proof_reference_parsing_is_strict() {
    let r = parse_proof_reference("CYNIC@2affadd8:docs/a.md").unwrap();
    assert_eq!((r.repo.as_str(), r.rev.as_str(), r.path.as_str()), ("CYNIC", "2affadd8", "docs/a.md"));
    for bad in ["", "CYNIC", "CYNIC@2affadd8", "@2affadd8:a", "CYNIC@:a", "CYNIC@zzzzzzzz:a", "CYNIC@abc:a", "CYNIC@2affadd8:"] {
        assert!(parse_proof_reference(bad).is_err(), "{bad:?} must be rejected");
    }
}

#[test]
fn verdicts_separate_verified_unresolved_and_undeclared() {
    let events = [
        event("ok", Some("CYNIC@aaaaaaa:x")),
        event("missing", Some("CYNIC@bbbbbbb:x")),
        event("malformed", Some("nonsense")),
        event("declared", None),
    ];
    let verdicts = verify_event_proofs(&events, |r| if r.rev == "aaaaaaa" { Ok(()) } else { Err("absent".to_string()) });
    assert_eq!(verdicts[0].outcome, ProofOutcome::Verified);
    assert!(matches!(verdicts[1].outcome, ProofOutcome::Unresolved(_)));
    assert!(matches!(verdicts[2].outcome, ProofOutcome::Unresolved(_)));
    assert_eq!(verdicts[3].outcome, ProofOutcome::Undeclared);
    assert_eq!(verdicts.iter().filter(|v| v.is_failure()).count(), 2);
}

#[test]
fn epoch_seconds_matches_known_instants() {
    assert_eq!(epoch_seconds("1970-01-01T00:00:00Z").unwrap(), 0);
    assert_eq!(epoch_seconds("2000-03-01T00:00:00Z").unwrap(), 951_868_800);
    assert_eq!(epoch_seconds("2026-10-25T10:21:35Z").unwrap(), 1_792_923_695);
    for bad in ["", "2026-10-25", "2026-13-01T00:00:00Z", "2026-10-25 10:21:35Z", "2026-10-25T10:21:35"] {
        assert!(epoch_seconds(bad).is_err(), "{bad:?} must be rejected");
    }
}

#[test]
fn freshness_gate_rejects_stale_and_future_surfaces() {
    let now = "2026-10-25T12:00:00Z";
    assert_eq!(check_freshness("2026-10-25T11:00:00Z", now, 3600).unwrap(), 3600);
    assert!(check_freshness("2026-10-25T10:59:59Z", now, 3600).is_err());
    assert!(check_freshness("2026-10-25T12:00:01Z", now, 3600).is_err());
    assert!(check_freshness("garbage", now, 3600).is_err());
}

#[test]
fn declared_profile_proofs_resolve_against_this_checkout() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
    let profile = cynic_knowledge::load_author_profile(repo_root).expect("author profile must load");
    let verdicts = verify_event_proofs(&profile.events, git_proof_resolver(repo_root));
    for v in &verdicts {
        assert!(!v.is_failure(), "{}: {:?}", v.event, v.outcome);
    }
}
