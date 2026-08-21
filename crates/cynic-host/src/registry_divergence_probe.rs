//! Probe the substrate for registry claims, through an injected argv runner.
//!
//! The decision lives in `cynic_protocol::registry_claim_audit`; this module
//! only observes and never judges. It is read-only by construction: every argv
//! it issues is a query (`ping -c1`, `getent hosts`, `curl -o /dev/null`), so a
//! divergence audit can never mutate the fleet it is auditing.
//!
//! Commands go through `ArgvRunner` rather than `std::process::Command`, so the
//! probes are unit-testable with `MapArgvRunner` and no live network. That
//! matters here more than usual: this code exists to distinguish "the claim is
//! false" from "I could not check", and a test suite that needed real peers
//! could never exercise the second case deliberately.

use cynic_protocol::registry_claim_audit::{ClaimKind, ClaimObservation, NameObservation};

use crate::update_lifecycle::ArgvRunner;

/// A claim to check, extracted from the registry by the caller.
#[derive(Debug, Clone)]
pub struct ClaimToProbe {
    pub node_id: String,
    pub kind: ClaimKind,
    /// The value the registry declares.
    pub declared: String,
    /// Whether the registry marks this entry `status: candidate`.
    pub declared_as_candidate: bool,
}

/// Probe whether a declared address answers.
///
/// A silent host yields `Unobservable`, not a refutation: ICMP may be filtered,
/// the host may be powered off, and neither proves the registry wrong. Only a
/// *different* answer refutes an address claim, which for reachability means the
/// caller must compare against a known-good value rather than mere silence.
pub fn probe_address_reachable(runner: &dyn ArgvRunner, claim: &ClaimToProbe) -> ClaimObservation {
    let output = runner.run(&["ping", "-c", "1", "-W", "2", &claim.declared]);
    let stderr = output.stderr.to_ascii_lowercase();

    // `ping` needs cap_net_raw. Unprivileged — the normal case for the `cynic`
    // principal — it exits non-zero with "operation not permitted": the probe was
    // denied and the host said nothing either way. Reading that as silence marks
    // every reachable node unobservable, which is exactly how this surface first
    // mis-graded cynic-builder against its own address.
    let privilege_denied = stderr.contains("operation not permitted")
        || stderr.contains("cap_net_raw")
        || stderr.contains("socket: permission denied");

    let (observed, reason) = if output.spawn_error.is_some() {
        (
            None,
            Some(format!(
                "ping unavailable: {}",
                output.spawn_error.clone().unwrap_or_default()
            )),
        )
    } else if privilege_denied {
        (
            None,
            Some(format!(
                "ICMP probe denied (missing cap_net_raw), so {} was never actually \
                 asked; this is a probe-privilege gap, not a silent host",
                claim.declared
            )),
        )
    } else if output.exit_code == Some(0) {
        // It answered at the declared address: the claim holds.
        (Some(claim.declared.clone()), None)
    } else {
        (
            None,
            Some(format!(
                "{} did not answer ICMP; may be filtered or powered off, which \
                 does not prove the registry wrong",
                claim.declared
            )),
        )
    };

    ClaimObservation {
        node_id: claim.node_id.clone(),
        kind: claim.kind,
        declared: claim.declared.clone(),
        observed,
        declared_as_candidate: claim.declared_as_candidate,
        unobservable_reason: reason,
    }
}

/// Probe whether a declared address answers a TCP connect, without privilege.
///
/// ICMP needs `cap_net_raw`, which the `cynic` principal does not hold, so
/// `probe_address_reachable` degrades to `Unobservable` on every node it cannot
/// ping — an honest verdict that nonetheless proves nothing. A TCP connect to a
/// port the node is expected to serve needs no capability at all, so it can
/// actually confirm the claim.
///
/// Uses `getent`-free, resolver-free argv (`bash -c` writing to `/dev/tcp`) so
/// it stays inside the injected `ArgvRunner` and remains unit-testable. Read-only:
/// opening and closing a connection mutates nothing.
///
/// A refused or timed-out port is still `Unobservable`, not a refutation: the
/// host may serve nothing on that port while being perfectly present at the
/// declared address. Only a *different* answer refutes an address claim.
pub fn probe_address_tcp(
    runner: &dyn ArgvRunner,
    claim: &ClaimToProbe,
    port: u16,
) -> ClaimObservation {
    let probe = format!("exec 3<>/dev/tcp/{}/{}", claim.declared, port);
    let output = runner.run(&["timeout", "2", "bash", "-c", &probe]);

    let (observed, reason) = if let Some(error) = &output.spawn_error {
        (None, Some(format!("tcp probe unavailable: {error}")))
    } else if output.exit_code == Some(0) {
        // It accepted a connection at the declared address: the claim holds, and
        // no privilege was required to establish it.
        (Some(claim.declared.clone()), None)
    } else {
        (
            None,
            Some(format!(
                "{}:{} refused or timed out; the node may serve nothing on that \
                 port while still holding the declared address, which does not \
                 prove the registry wrong",
                claim.declared, port
            )),
        )
    };

    ClaimObservation {
        node_id: claim.node_id.clone(),
        kind: claim.kind,
        declared: claim.declared.clone(),
        observed,
        declared_as_candidate: claim.declared_as_candidate,
        unobservable_reason: reason,
    }
}

/// Resolve a name from this vantage point.
///
/// Returns the first address `getent hosts` prints. A caller building a
/// `NameObservation` must gather one of these per vantage point; a single
/// resolver cannot detect an inconsistency.
pub fn resolve_name_here(
    runner: &dyn ArgvRunner,
    observer_id: &str,
    name: &str,
) -> Result<(String, String), (String, String)> {
    let output = runner.run(&["getent", "hosts", name]);

    if let Some(error) = &output.spawn_error {
        return Err((
            observer_id.to_string(),
            format!("getent unavailable: {error}"),
        ));
    }
    if output.exit_code != Some(0) {
        return Err((observer_id.to_string(), "name does not resolve".to_string()));
    }

    match output
        .stdout
        .split_whitespace()
        .next()
        .filter(|value| !value.is_empty())
    {
        Some(address) => Ok((observer_id.to_string(), address.to_string())),
        None => Err((
            observer_id.to_string(),
            "getent exited 0 with no address".to_string(),
        )),
    }
}

/// Assemble a multi-vantage name observation from per-observer results.
///
/// Separating resolution from assembly keeps the vantage points injectable: one
/// may come from this host, another over ssh, another from a registry document,
/// and the decision layer never learns which.
pub fn assemble_name_observation(
    name: &str,
    results: Vec<Result<(String, String), (String, String)>>,
) -> NameObservation {
    let mut answers = Vec::new();
    let mut unresolved = Vec::new();
    for result in results {
        match result {
            Ok(answer) => answers.push(answer),
            Err(failure) => unresolved.push(failure),
        }
    }
    NameObservation {
        name: name.to_string(),
        answers,
        unresolved,
    }
}

/// Probe whether a declared service endpoint is listening.
///
/// An HTTP status — any status — proves something is bound there, so `401` is a
/// confirmation, not a failure. The kernel on cortex answers exactly that, and
/// treating auth-required as down would have hidden it.
pub fn probe_service_binding(
    runner: &dyn ArgvRunner,
    claim: &ClaimToProbe,
    url: &str,
) -> ClaimObservation {
    let output = runner.run(&[
        "curl",
        "-s",
        "-o",
        "/dev/null",
        "-w",
        "%{http_code}",
        "--max-time",
        "4",
        url,
    ]);

    let code = output.stdout.trim().to_string();
    let (observed, reason) = if output.spawn_error.is_some() {
        (
            None,
            Some(format!(
                "curl unavailable: {}",
                output.spawn_error.clone().unwrap_or_default()
            )),
        )
    } else if code.is_empty() || code == "000" {
        (
            None,
            Some(format!(
                "no HTTP response from {url}; endpoint may be down or filtered"
            )),
        )
    } else {
        // Any status proves a listener exists at the declared endpoint.
        (Some(claim.declared.clone()), None)
    };

    ClaimObservation {
        node_id: claim.node_id.clone(),
        kind: claim.kind,
        declared: claim.declared.clone(),
        observed,
        declared_as_candidate: claim.declared_as_candidate,
        unobservable_reason: reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};
    use cynic_protocol::registry_claim_audit::{ClaimVerdict, judge_claim, judge_name};

    fn ok(stdout: &str) -> ArgvOutput {
        ArgvOutput {
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
            spawn_error: None,
        }
    }

    fn failed(code: i32) -> ArgvOutput {
        ArgvOutput {
            exit_code: Some(code),
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: None,
        }
    }

    fn missing_tool() -> ArgvOutput {
        ArgvOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: Some("no such file or directory".to_string()),
        }
    }

    fn runner(argv: &[&str], output: ArgvOutput) -> MapArgvRunner {
        MapArgvRunner {
            responses: vec![(argv.iter().map(|s| (*s).to_string()).collect(), output)],
        }
    }

    fn claim(declared: &str, candidate: bool) -> ClaimToProbe {
        ClaimToProbe {
            node_id: "local-kernel".to_string(),
            kind: ClaimKind::SynapseAddress,
            declared: declared.to_string(),
            declared_as_candidate: candidate,
        }
    }

    /// The live case: 10.10.10.100 is declared and answers nothing.
    ///
    /// Silence must stay UNOBSERVABLE. ICMP may be filtered and the host may be
    /// powered off — neither proves the registry wrong, and reporting silence as
    /// a refutation would manufacture divergences across a firewalled fleet.
    #[test]
    fn a_silent_declared_address_is_unobservable_not_refuted() {
        let r = runner(&["ping", "-c", "1", "-W", "2", "10.10.10.100"], failed(1));
        let observation = probe_address_reachable(&r, &claim("10.10.10.100", true));

        assert!(observation.observed.is_none());
        let reason = observation
            .unobservable_reason
            .as_deref()
            .expect("silence must explain itself");
        assert!(reason.contains("filtered") || reason.contains("powered off"));

        match judge_claim(&observation) {
            ClaimVerdict::Unobservable { .. } => {}
            other => panic!("silence must not decide the claim: {other:?}"),
        }
    }

    /// A denied probe is not a silent host.
    ///
    /// Found by running the surface for real: under the `cynic` principal `ping`
    /// exits 2 with "Operation not permitted" (no cap_net_raw), and the first
    /// version read that as silence — grading cynic-builder unobservable against
    /// its OWN address, which it answers instantly. Same verdict class, but the
    /// reason must name the privilege gap or the operator chases a network fault
    /// that does not exist.
    #[test]
    fn a_denied_icmp_probe_reports_the_privilege_gap_not_silence() {
        let denied = ArgvOutput {
            exit_code: Some(2),
            stdout: String::new(),
            stderr: "ping: socket: Operation not permitted\nping: => missing cap_net_raw+p capability or setuid?".to_string(),
            spawn_error: None,
        };
        let r = runner(&["ping", "-c", "1", "-W", "2", "10.10.10.119"], denied);
        let observation = probe_address_reachable(&r, &claim("10.10.10.119", true));

        let reason = observation
            .unobservable_reason
            .as_deref()
            .expect("a denied probe must explain itself");
        assert!(
            reason.contains("cap_net_raw") && reason.contains("never actually"),
            "must name the privilege gap, got: {reason}"
        );
        assert!(
            !reason.contains("powered off"),
            "must not blame the host for a local privilege gap: {reason}"
        );
        // Still unobservable — but now for the right reason.
        match judge_claim(&observation) {
            ClaimVerdict::Unobservable { .. } => {}
            other => panic!("expected Unobservable, got {other:?}"),
        }
    }

    #[test]
    fn an_answering_declared_address_confirms_the_claim() {
        let r = runner(&["ping", "-c", "1", "-W", "2", "10.10.10.119"], ok(""));
        let observation = probe_address_reachable(&r, &claim("10.10.10.119", false));
        assert_eq!(judge_claim(&observation), ClaimVerdict::Confirmed);
    }

    /// A missing tool is an observation gap, never a fleet fault.
    #[test]
    fn a_missing_probe_tool_is_unobservable() {
        let r = runner(&["ping"], missing_tool());
        let observation = probe_address_reachable(&r, &claim("10.10.10.100", false));
        let reason = observation.unobservable_reason.expect("reason");
        assert!(reason.contains("ping unavailable"));
    }

    /// The kernel on cortex answers 401 without a credential.
    ///
    /// Any HTTP status proves a listener is bound, so 401 CONFIRMS the binding.
    /// Treating auth-required as down would have hidden the running kernel and
    /// reported the fleet as missing its primary role.
    #[test]
    fn an_auth_required_endpoint_still_proves_a_listener() {
        let url = "http://192.168.100.2:3030/health";
        let r = runner(
            &[
                "curl",
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "--max-time",
                "4",
                url,
            ],
            ok("401"),
        );
        let observation = probe_service_binding(&r, &claim("192.168.100.2:3030", false), url);
        assert_eq!(judge_claim(&observation), ClaimVerdict::Confirmed);
    }

    /// curl prints 000 when nothing answered — that is unknown, not proof.
    #[test]
    fn no_http_response_is_unobservable() {
        let url = "http://10.10.10.100:3030/health";
        let r = runner(&["curl"], ok("000"));
        let observation = probe_service_binding(&r, &claim("10.10.10.100:3030", true), url);
        assert!(observation.observed.is_none());
        assert!(
            observation
                .unobservable_reason
                .expect("reason")
                .contains("no HTTP response")
        );
    }

    /// getent output is parsed to its first field: the address.
    #[test]
    fn resolving_a_name_yields_its_address() {
        let r = runner(
            &["getent", "hosts", "kernel.synapse.cynic"],
            ok("192.168.100.2   kernel.synapse.cynic local-kernel.synapse.cynic"),
        );
        let result = resolve_name_here(&r, "cynic-builder", "kernel.synapse.cynic");
        assert_eq!(
            result,
            Ok(("cynic-builder".to_string(), "192.168.100.2".to_string()))
        );
    }

    /// A name that does not resolve is recorded against its observer, so the
    /// decision layer can tell "nobody could check" from "they disagreed".
    #[test]
    fn an_unresolvable_name_is_attributed_to_its_observer() {
        let r = runner(&["getent", "hosts", "cortex.synapse.cynic"], failed(2));
        let result = resolve_name_here(&r, "cynic-builder", "cortex.synapse.cynic");
        assert_eq!(
            result,
            Err((
                "cynic-builder".to_string(),
                "name does not resolve".to_string()
            ))
        );
    }

    /// Exit 0 with empty output is a gap, not an address.
    #[test]
    fn getent_success_without_an_address_is_a_failure_to_resolve() {
        let r = runner(&["getent"], ok("   "));
        let result = resolve_name_here(&r, "cortex", "kernel.synapse.cynic");
        assert!(result.is_err());
    }

    /// Assembling the live disagreement: cortex says 127.0.0.1, the builder says
    /// 192.168.100.2, forgejo cannot resolve at all. Two answers that differ is
    /// an inconsistency; the third stays unknown.
    #[test]
    fn assembled_vantage_points_expose_the_live_name_conflict() {
        let observation = assemble_name_observation(
            "kernel.synapse.cynic",
            vec![
                Ok(("cortex".to_string(), "127.0.0.1".to_string())),
                Ok(("cynic-builder".to_string(), "192.168.100.2".to_string())),
                Err(("forgejo".to_string(), "name does not resolve".to_string())),
            ],
        );

        assert_eq!(observation.answers.len(), 2);
        assert_eq!(observation.unresolved.len(), 1);

        match judge_name(&observation) {
            cynic_protocol::registry_claim_audit::NameVerdict::Inconsistent { distinct_values } => {
                assert_eq!(distinct_values.len(), 2)
            }
            other => panic!("expected Inconsistent, got {other:?}"),
        }
    }

    /// Every probe argv must be read-only: an audit may never mutate the fleet
    /// it audits. This pins the commands so a future edit cannot slip in a
    /// mutating flag unnoticed.
    #[test]
    fn probes_issue_only_read_only_commands() {
        let mutating = [
            "ip",
            "systemctl",
            "rm",
            "mv",
            "install",
            "tee",
            "sed",
            "apt",
            "nmcli",
        ];
        for argv in [
            vec!["ping", "-c", "1", "-W", "2", "10.10.10.100"],
            vec!["getent", "hosts", "kernel.synapse.cynic"],
            vec!["curl", "-s", "-o", "/dev/null", "-w", "%{http_code}"],
            vec![
                "timeout",
                "2",
                "bash",
                "-c",
                "exec 3<>/dev/tcp/10.10.10.119/22",
            ],
        ] {
            // argv[0] alone stopped being sufficient once a probe shells out:
            // `bash -c` can carry anything, so check every word.
            for word in &argv {
                assert!(!mutating.contains(word), "{word} is not a read-only probe");
            }
        }
    }

    /// The whole point of the TCP probe: confirm an address with no capability.
    ///
    /// `probe_address_reachable` cannot do this for the `cynic` principal, which
    /// holds no `cap_net_raw`, so every node it audits degrades to Unobservable.
    #[test]
    fn a_tcp_connect_confirms_an_address_without_privilege() {
        let claim = ClaimToProbe {
            node_id: "cynic-builder".to_string(),
            kind: ClaimKind::SynapseAddress,
            declared: "10.10.10.119".to_string(),
            declared_as_candidate: false,
        };
        let r = runner(
            &[
                "timeout",
                "2",
                "bash",
                "-c",
                "exec 3<>/dev/tcp/10.10.10.119/22",
            ],
            ok(""),
        );

        let observation = probe_address_tcp(&r, &claim, 22);

        assert_eq!(observation.observed.as_deref(), Some("10.10.10.119"));
        assert_eq!(judge_claim(&observation), ClaimVerdict::Confirmed);
        assert!(observation.unobservable_reason.is_none());
    }

    /// A closed port is not a refutation.
    ///
    /// The node may hold the declared address while serving nothing there, so
    /// the honest verdict is Unobservable — the same discipline the ICMP probe
    /// applies to silence.
    #[test]
    fn a_closed_port_stays_unobservable_not_refuted() {
        let claim = ClaimToProbe {
            node_id: "local-kernel".to_string(),
            kind: ClaimKind::SynapseAddress,
            declared: "10.10.10.100".to_string(),
            declared_as_candidate: true,
        };
        let r = runner(
            &[
                "timeout",
                "2",
                "bash",
                "-c",
                "exec 3<>/dev/tcp/10.10.10.100/22",
            ],
            failed(1),
        );

        let observation = probe_address_tcp(&r, &claim, 22);

        assert!(observation.observed.is_none());
        assert!(
            observation
                .unobservable_reason
                .as_deref()
                .unwrap_or_default()
                .contains("does not prove the registry wrong"),
            "a closed port must not read as a refutation"
        );
    }
}
