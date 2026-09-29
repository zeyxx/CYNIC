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

use cynic_host::audit_engine::HostObservation;

const SECRET_VALUE: &str = "s3cr3t-value-that-must-never-appear-in-a-report-0000";

fn observation_with_units(files: &[(&str, &str)], exec_starts: &[(&str, &str)]) -> HostObservation {
    let mut observation = HostObservation::default();
    for (name, text) in files {
        let path = PathBuf::from("/etc/systemd/system").join(name);
        observation
            .unit_files
            .push(path.clone());
        observation.unit_contents.insert(path, (*text).to_string());
    }
    for (unit, program) in exec_starts {
        observation
            .exec_starts
            .push(((*unit).to_string(), PathBuf::from(program)));
    }
    observation.unit_files.sort();
    observation.exec_starts.sort();
    observation
}

/// A host observation over the fake units, with an empty registry so every fake
/// unit is "owned" by the project's naming space and therefore in scope.
fn evaluate(
    files: &[(&str, &str)],
    exec_starts: &[(&str, &str)],
) -> Vec<cynic_protocol::AuditViolation> {
    let index = cynic_host::audit_engine::WorkspaceSourceIndex {
        host: Some(observation_with_units(files, exec_starts)),
        ..Default::default()
    };
    cynic_host::audit_engine::evaluate_runtime_divergence(&index)
}

fn codes(violations: &[cynic_protocol::AuditViolation]) -> BTreeSet<String> {
    violations.iter().map(|v| v.code.clone()).collect()
}

#[test]
fn a_credential_in_a_unit_is_found_and_its_value_is_never_reported() {
    let unit = format!(
        "[Service]\nEnvironment=\"CYNIC_API_KEY_INTERNAL={SECRET_VALUE}\"\n"
    );
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
    let unit = "[Service]\nEnvironment=\"CYNIC_API_KEY_INTERNAL=/etc/cynic/secrets/kernel-runtime.env\"\n";
    let codes = codes(&evaluate(&[("cynic-fixture.service", unit)], &[]));
    assert!(
        !codes.contains("RUN-001-CREDENTIAL-IN-UNIT"),
        "a path to a principal-owned file is the fix, not a leak; codes were {codes:?}"
    );
}

#[test]
fn a_short_or_uncredential_shaped_value_is_not_a_finding() {
    let unit = "[Service]\nEnvironment=\"CYNIC_NODE_ID=local-kernel\"\nEnvironment=\"CYNIC_PORT=3030\"\n";
    let codes = codes(&evaluate(&[("cynic-fixture.service", unit)], &[]));
    assert!(
        !codes.contains("RUN-001-CREDENTIAL-IN-UNIT"),
        "a node id and a port are not credentials; codes were {codes:?}"
    );
}

#[test]
fn a_unit_whose_execstart_is_absent_is_found() {
    // 203/EXEC: systemd retries this on a timer forever and reports nothing a
    // human reads. One unit had 3092 recorded restarts.
    let codes = codes(&evaluate(
        &[("cynic-fixture.service", "[Service]\n")],
        &[("cynic-fixture.service", "/usr/local/lib/cynic/absent-binary")],
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
        !codes.contains("RUN-001-CYNIC-UNIT-UNDECLARED") && !codes.contains("RUN-001-EXECSTART-ABSENT"),
        "an operating-system unit is out of scope; codes were {codes:?}"
    );
}

#[test]
fn a_project_binary_without_provenance_is_found() {
    let codes = codes(&evaluate(
        &[("cynic-fixture.service", "[Service]\n")],
        &[("cynic-fixture.service", "/usr/local/bin/cynic-kernel")],
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
    let index = cynic_host::audit_engine::WorkspaceSourceIndex {
        host: None,
        ..Default::default()
    };
    let violations = cynic_host::audit_engine::evaluate_runtime_divergence(&index);
    assert!(
        violations.is_empty(),
        "with no observation the rule must return nothing rather than guess"
    );
}
