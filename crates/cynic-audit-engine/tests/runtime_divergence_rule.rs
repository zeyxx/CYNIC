//! RUN-001, proven against fixtures instead of against whichever host runs it.
//!
//! WHY A FIXTURE AND NOT THE REAL HOST. The credential check is the one that
//! must never be wrong in the direction that matters: a false negative means a
//! cleartext secret survives, and a false positive that printed the value would
//! leak it into a report, a log and a transcript. Neither can be demonstrated by
//! pointing the rule at a machine you happen to be on, because on this host the
//! check currently finds nothing — the values were rotated into a 0640 file — and
//! "found nothing" is not evidence that it can find something.
//!
//! So the rule is exercised on hand-built observations. The scanner keeps the
//! real I/O; the rule keeps the logic; this file holds both to the behaviour that
//! is worth trusting. No environment variable, no `/etc`, no host.

#![allow(clippy::expect_used)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use cynic_audit_engine::HostObservation;

const SECRET_VALUE: &str = "s3cr3t-value-that-must-never-appear-in-a-report-0000";

/// Build a host observation.
///
/// `present` names the ExecStart targets that exist, and `provenanced` the subset
/// that carry a provenance record. They are stated here rather than looked up,
/// because the rule compares what it is given instead of inspecting the machine:
/// a rule that calls `Path::exists` reports on the host it runs on, which is a
/// fact about no fixture and no other machine.
fn observation_with(
    files: &[(&str, &str)],
    exec_starts: &[(&str, &str)],
    present: &[&str],
    provenanced: &[&str],
) -> HostObservation {
    let mut observation = HostObservation::default();
    for (name, text) in files {
        let path = PathBuf::from("/etc/systemd/system").join(name);
        observation.unit_files.push(path.clone());
        observation.unit_contents.insert(path, (*text).to_string());
    }
    for (unit, program) in exec_starts {
        observation
            .exec_starts
            .push(((*unit).to_string(), PathBuf::from(program)));
    }
    for path in present {
        observation.existing_exec_starts.insert(PathBuf::from(path));
    }
    for path in provenanced {
        observation
            .provenanced_exec_starts
            .insert(PathBuf::from(path));
    }
    observation.unit_files.sort();
    observation.exec_starts.sort();
    observation
}

/// The common case: every named ExecStart target exists and none is provenanced.
fn observation_with_units(files: &[(&str, &str)], exec_starts: &[(&str, &str)]) -> HostObservation {
    let present: Vec<&str> = exec_starts.iter().map(|(_, p)| *p).collect();
    observation_with(files, exec_starts, &present, &[])
}

/// A host observation over the fake units, with an empty registry so every fake
/// unit is "owned" by the project's naming space and therefore in scope.
fn evaluate_with(
    files: &[(&str, &str)],
    exec_starts: &[(&str, &str)],
    present: &[&str],
    provenanced: &[&str],
) -> Vec<cynic_protocol::AuditViolation> {
    let index = cynic_audit_engine::WorkspaceSourceIndex {
        host: Some(observation_with(files, exec_starts, present, provenanced)),
        ..Default::default()
    };
    cynic_audit_engine::evaluate_runtime_divergence(&index)
}

/// Every named target exists and none carries a provenance record.
fn evaluate(
    files: &[(&str, &str)],
    exec_starts: &[(&str, &str)],
) -> Vec<cynic_protocol::AuditViolation> {
    evaluate_with(
        files,
        exec_starts,
        &exec_starts.iter().map(|(_, p)| *p).collect::<Vec<_>>(),
        &[],
    )
}

fn codes(violations: &[cynic_protocol::AuditViolation]) -> BTreeSet<String> {
    violations.iter().map(|v| v.code.clone()).collect()
}

#[test]
fn a_credential_in_a_unit_is_found_and_its_value_is_never_reported() {
    let unit = format!("[Service]\nEnvironment=\"CYNIC_API_KEY_INTERNAL={SECRET_VALUE}\"\n");
    let violations = evaluate(&[("cynic-fixture.service", unit.as_str())], &[]);
    let codes = codes(&violations);

    assert!(
        codes.contains("RUN-001-CREDENTIAL-IN-UNIT"),
        "a cleartext credential in a unit file must be found; codes were {codes:?}"
    );

    let report = violations
        .iter()
        .map(|v| format!("{} {} {:?}", v.code, v.message, v.remediation))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        !report.contains(SECRET_VALUE),
        "the report leaked the secret value. A check that prints the secret is worse \
         than no check, because the secret then lives in every log and transcript that \
         carries the report."
    );
    // The variable NAME is what makes the finding actionable, so it must be there.
    assert!(
        report.contains("CYNIC_API_KEY_INTERNAL"),
        "the report must name the variable so an operator can find it"
    );
}

#[test]
fn a_credential_file_reference_is_not_a_finding() {
    // The correct shape: the unit points at a file the principal owns.
    let unit =
        "[Service]\nEnvironment=\"CYNIC_API_KEY_INTERNAL=/etc/cynic/secrets/kernel-runtime.env\"\n";
    let codes = codes(&evaluate(&[("cynic-fixture.service", unit)], &[]));
    assert!(
        !codes.contains("RUN-001-CREDENTIAL-IN-UNIT"),
        "a path to a principal-owned file is the fix, not a leak; codes were {codes:?}"
    );
}

#[test]
fn a_short_or_uncredential_shaped_value_is_not_a_finding() {
    let unit =
        "[Service]\nEnvironment=\"CYNIC_NODE_ID=local-kernel\"\nEnvironment=\"CYNIC_PORT=3030\"\n";
    let codes = codes(&evaluate(&[("cynic-fixture.service", unit)], &[]));
    assert!(
        !codes.contains("RUN-001-CREDENTIAL-IN-UNIT"),
        "a node id and a port are not credentials; codes were {codes:?}"
    );
}

#[test]
fn a_unit_whose_execstart_is_absent_is_found() {
    // `present` is empty here: the named binary does not exist on this host,
    // which is the divergence, and the fixture now says so rather than leaving
    // the rule to discover it.
    // 203/EXEC: systemd retries this on a timer forever and reports nothing a
    // human reads. One unit had 3092 recorded restarts.
    let codes = codes(&evaluate_with(
        &[("cynic-fixture.service", "[Service]\n")],
        &[(
            "cynic-fixture.service",
            "/usr/local/lib/cynic/absent-binary",
        )],
        &[],
        &[],
    ));
    assert!(
        codes.contains("RUN-001-EXECSTART-ABSENT"),
        "a unit naming a binary that does not exist must be found; codes were {codes:?}"
    );
}

#[test]
fn a_unit_outside_the_project_naming_space_is_out_of_scope() {
    // The first version of this rule reported all 326 units on the host,
    // including systemd-journald. Demanding provenance of the operating system
    // is not a CYNIC divergence, and a rule that flags it is one nobody reads.
    let codes = codes(&evaluate(
        &[("apparmor.service", "[Service]\n")],
        &[("apparmor.service", "/usr/sbin/apparmor_parser")],
    ));
    assert!(
        !codes.contains("RUN-001-CYNIC-UNIT-UNDECLARED")
            && !codes.contains("RUN-001-EXECSTART-ABSENT"),
        "an operating-system unit is out of scope; codes were {codes:?}"
    );
}

#[test]
fn a_project_binary_without_provenance_is_found() {
    // The target is stated present. An earlier version named
    // /usr/local/bin/cynic-kernel and relied on the rule calling
    // `Path::exists` against the builder, so the proof only held on a machine
    // with that binary installed, and failed on every other host.
    let codes = codes(&evaluate_with(
        &[("cynic-fixture.service", "[Service]\n")],
        &[("cynic-fixture.service", "/usr/local/bin/cynic-fixture")],
        &["/usr/local/bin/cynic-fixture"],
        &[],
    ));
    assert!(
        codes.contains("RUN-001-NO-PROVENANCE"),
        "a binary this project built, with no commit and no digest, is the divergence \
         that made a 49-day-old service look healthy; codes were {codes:?}"
    );
}

#[test]
fn no_host_observed_means_no_claim_of_clean() {
    // A host that was not observed must produce nothing, and the rule must not
    // pretend otherwise. `None` is the shape an unknown takes here.
    let index = cynic_audit_engine::WorkspaceSourceIndex {
        host: None,
        ..Default::default()
    };
    let violations = cynic_audit_engine::evaluate_runtime_divergence(&index);
    assert!(
        violations.is_empty(),
        "with no observation the rule must return nothing rather than guess"
    );
}

/// The seam: the observation is POPULATED from a real filesystem, not supplied.
///
/// Every other test in this file builds a `HostObservation` by hand, which means
/// each one hands its rule the answer that rule is about to read. Breaking
/// `record_exec_start_facts` entirely would leave all of them passing — verified:
/// disabling the existence check left this file green. A rule whose input is
/// fabricated by its own test is not being tested at all.
///
/// So this test drives the population step against a temporary directory: a unit
/// naming a real binary, a unit naming a binary with a `.commit` beside it, and a
/// unit naming one that is not there. Each must land where it belongs, because
/// those two sets are the only thing that tells the rule which divergence it is
/// looking at.
#[test]
fn the_observation_is_populated_from_the_filesystem_not_assumed() {
    use cynic_audit_engine::scanner::record_exec_start_facts;
    use std::fs;

    let dir = std::env::temp_dir().join(format!("cynic-run001-seam-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture dir");

    // present, no provenance record
    let bare = dir.join("cynic-bare");
    fs::write(&bare, b"#!/bin/sh\n").expect("write binary");
    // present, with a commit record beside it
    let tracked = dir.join("cynic-tracked");
    fs::write(&tracked, b"#!/bin/sh\n").expect("write binary");
    fs::write(dir.join("cynic-tracked.commit"), b"abc123\n").expect("write provenance");
    // named but absent
    let missing = dir.join("cynic-missing");

    let mut observation = HostObservation {
        exec_starts: vec![
            ("bare.service".to_string(), bare.clone()),
            ("tracked.service".to_string(), tracked.clone()),
            ("missing.service".to_string(), missing.clone()),
        ],
        ..HostObservation::default()
    };

    record_exec_start_facts(&mut observation);

    assert!(
        observation.existing_exec_starts.contains(&bare),
        "a binary that exists must be recorded as existing, or the rule cannot tell \
         an absent ExecStart from a present one"
    );
    assert!(
        observation.existing_exec_starts.contains(&tracked),
        "a binary with a provenance record still exists and must be in the same set"
    );
    assert!(
        !observation.existing_exec_starts.contains(&missing),
        "a binary that is not on disk must not be recorded as existing"
    );
    assert!(
        !observation.provenanced_exec_starts.contains(&bare),
        "a binary with no record beside it must not be recorded as provenanced, or \
         RUN-001-NO-PROVENANCE can never fire"
    );
    assert!(
        observation.provenanced_exec_starts.contains(&tracked),
        "a `.commit` file beside the binary is the provenance the rule looks for"
    );

    // And the rule, fed exactly that observation, must report the bare one and
    // not the tracked one. Without this the seam is proven but the consequence
    // is not.
    // The rule scopes itself to CYNIC install prefixes
    // (CYNIC_BINARY_PREFIXES: /usr/local/bin/cynic, /usr/local/lib/cynic,
    // /opt/cynic), so a temporary directory is out of scope by design — a rule
    // that flagged every file on the host would be a rule nobody reads. The
    // consequence is therefore proven on the SCOPE test that already exists,
    // not here: this test owns the population step, and the two together cover
    // the rule.
    //
    // Asserted here so the limit is recorded rather than discovered later: the
    // binary is real and lacks provenance, and it is deliberately NOT reported,
    // because it is not in a CYNIC prefix.
    let codes = codes(&evaluate_with(
        &[],
        &[
            ("bare.service", bare.to_str().unwrap()),
            ("tracked.service", tracked.to_str().unwrap()),
        ],
        &[bare.to_str().unwrap(), tracked.to_str().unwrap()],
        &[],
    ));
    assert!(
        !codes.contains("RUN-001-NO-PROVENANCE"),
        "a binary outside the CYNIC install prefixes must stay out of scope; a rule \
         that flags every executable on the host is a rule nobody reads. \
         codes: {codes:?}"
    );

    let _ = fs::remove_dir_all(&dir);
}
