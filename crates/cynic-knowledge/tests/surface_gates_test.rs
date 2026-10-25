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

fn claim(id: &str, cmd: Option<&str>, value: Option<u64>) -> cynic_knowledge::VerifiableClaim {
    cynic_knowledge::VerifiableClaim {
        id: id.to_string(),
        category: cynic_knowledge::ClaimCategory::Architecture,
        statement: String::new(),
        status: cynic_knowledge::ClaimStatus::Demonstrated,
        evidence: String::new(),
        verify_command: cmd.map(str::to_string),
        numeric_value: value,
        unit: None,
    }
}

#[test]
fn claim_recheck_separates_match_mismatch_unverifiable_and_no_command() {
    use cynic_knowledge::{check_claims, ClaimCheckOutcome};
    let claims = [
        claim("same", Some("c1"), Some(5)),
        claim("drift", Some("c2"), Some(5)),
        claim("prose", Some("c3"), Some(5)),
        claim("broken", Some("c4"), Some(5)),
        claim("nonumber", Some("c5"), None),
        claim("nocmd", None, Some(5)),
    ];
    let checks = check_claims(&claims, |cmd| match cmd {
        "c1" => Ok("5\n".to_string()),
        "c2" => Ok("6".to_string()),
        "c3" => Ok("{\"a\":1}".to_string()),
        _ => Err("exit 1".to_string()),
    });
    assert_eq!(checks[0].outcome, ClaimCheckOutcome::Match);
    assert_eq!(checks[1].outcome, ClaimCheckOutcome::Mismatch { observed: 6 });
    assert!(matches!(checks[2].outcome, ClaimCheckOutcome::Unverifiable(_)));
    assert!(matches!(checks[3].outcome, ClaimCheckOutcome::Unverifiable(_)));
    assert!(matches!(checks[4].outcome, ClaimCheckOutcome::Unverifiable(_)));
    assert_eq!(checks[5].outcome, ClaimCheckOutcome::NoCommand);
    assert_eq!(checks.iter().filter(|c| c.is_failure()).count(), 1, "only a mismatch fails the gate");
}

#[test]
fn parse_count_accepts_only_one_whole_integer() {
    assert_eq!(cynic_knowledge::parse_count(" 42\n").unwrap(), 42);
    for bad in ["", "4 2", "-1", "4.0", "{\"n\":1}"] {
        assert!(cynic_knowledge::parse_count(bad).is_err(), "{bad:?}");
    }
}
