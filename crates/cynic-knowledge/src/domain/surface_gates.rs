//! Gates on the public epistemic surface.
//!
//! Pure rules, no I/O: how a proof reference is read, what a verdict says, and
//! whether a published surface is still fresh. Resolution of a reference against
//! a repository lives behind a closure supplied by an adapter.

use crate::domain::epistemic_facts::HistoricalEvent;

/// A pointer to a checkable artifact: `repo@revision:path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofRef {
    pub repo: String,
    pub rev: String,
    pub path: String,
}

/// Parses `repo@revision:path`. The revision must be a hexadecimal commit id (7 to 40 digits).
pub fn parse_proof_reference(raw: &str) -> Result<ProofRef, String> {
    let (head, path) = raw
        .split_once(':')
        .ok_or_else(|| format!("proof reference {raw:?}: expected repo@revision:path"))?;
    let (repo, rev) = head
        .split_once('@')
        .ok_or_else(|| format!("proof reference {raw:?}: expected repo@revision:path"))?;
    if repo.is_empty() || path.is_empty() {
        return Err(format!("proof reference {raw:?}: empty repo or path"));
    }
    if !(7..=40).contains(&rev.len()) || !rev.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("proof reference {raw:?}: revision {rev:?} is not a 7-40 digit commit id"));
    }
    Ok(ProofRef {
        repo: repo.to_string(),
        rev: rev.to_string(),
        path: path.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofOutcome {
    /// The referenced artifact exists at that revision.
    Verified,
    /// A reference is declared but cannot be resolved. This fails the gate.
    Unresolved(String),
    /// The event declares no reference: it is the author's statement, not a checked fact.
    Undeclared,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofVerdict {
    pub event: String,
    pub reference: Option<String>,
    pub outcome: ProofOutcome,
}

impl ProofVerdict {
    pub fn is_failure(&self) -> bool {
        matches!(self.outcome, ProofOutcome::Unresolved(_))
    }
}

/// Checks every event's `proof_reference` with the supplied resolver.
pub fn verify_event_proofs<F>(events: &[HistoricalEvent], resolve: F) -> Vec<ProofVerdict>
where
    F: Fn(&ProofRef) -> Result<(), String>,
{
    events
        .iter()
        .map(|event| {
            let outcome = match event.proof_reference.as_deref() {
                None => ProofOutcome::Undeclared,
                Some(raw) => match parse_proof_reference(raw).and_then(|r| resolve(&r)) {
                    Ok(()) => ProofOutcome::Verified,
                    Err(why) => ProofOutcome::Unresolved(why),
                },
            };
            ProofVerdict {
                event: event.title.clone(),
                reference: event.proof_reference.clone(),
                outcome,
            }
        })
        .collect()
}

/// Seconds since the Unix epoch for a `YYYY-MM-DDThh:mm:ssZ` UTC timestamp.
pub fn epoch_seconds(ts: &str) -> Result<i64, String> {
    let bad = || format!("timestamp {ts:?}: expected YYYY-MM-DDThh:mm:ssZ");
    let b = ts.as_bytes();
    if b.len() != 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' || b[19] != b'Z' {
        return Err(bad());
    }
    let num = |from: usize, to: usize| -> Result<i64, String> { ts[from..to].parse::<i64>().map_err(|_| bad()) };
    let (y, mo, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (h, mi, s) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return Err(bad());
    }
    // Days from civil date (proleptic Gregorian), 1970-01-01 = 0.
    let y = if mo <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Ok(days * 86_400 + h * 3_600 + mi * 60 + s)
}

/// Returns the age in seconds when the surface is within `max_age_secs`, else an error.
/// A timestamp in the future is an error too: a clock or a forgery, never "fresh".
pub fn check_freshness(generated_at: &str, now: &str, max_age_secs: u64) -> Result<u64, String> {
    let generated = epoch_seconds(generated_at)?;
    let now = epoch_seconds(now)?;
    if generated > now {
        return Err(format!("generated_at {generated_at} is in the future of {now}"));
    }
    let age = (now - generated) as u64;
    if age > max_age_secs {
        return Err(format!("surface is {age}s old, limit is {max_age_secs}s (generated_at {generated_at})"));
    }
    Ok(age)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimCheckOutcome {
    /// The command printed exactly the published number.
    Match,
    /// The command printed a different number. This fails the gate.
    Mismatch { observed: u64 },
    /// The claim cannot be re-measured here (command failed, printed no single count,
    /// or there is no number to compare). Reported, never counted as verified.
    Unverifiable(String),
    /// The claim declares no verify_command.
    NoCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimCheck {
    pub id: String,
    pub expected: Option<u64>,
    pub outcome: ClaimCheckOutcome,
}

impl ClaimCheck {
    pub fn is_failure(&self) -> bool {
        matches!(self.outcome, ClaimCheckOutcome::Mismatch { .. })
    }
}

/// A command output counts only when it is one whole unsigned integer.
pub fn parse_count(stdout: &str) -> Result<u64, String> {
    let trimmed = stdout.trim();
    trimmed
        .parse::<u64>()
        .map_err(|_| format!("output is not a single count: {:?}", trimmed.chars().take(60).collect::<String>()))
}

/// Re-runs each claim's `verify_command` through `run` and compares the printed count with
/// the published `numeric_value`. `run` must only receive commands of a ledger harvested in
/// this process, never commands read back from a file.
pub fn check_claims<F>(claims: &[crate::domain::epistemic_facts::VerifiableClaim], run: F) -> Vec<ClaimCheck>
where
    F: Fn(&str) -> Result<String, String>,
{
    claims
        .iter()
        .map(|claim| {
            let outcome = match (claim.verify_command.as_deref(), claim.numeric_value) {
                (None, _) => ClaimCheckOutcome::NoCommand,
                (Some(_), None) => ClaimCheckOutcome::Unverifiable("claim has no numeric value to compare".to_string()),
                (Some(cmd), Some(expected)) => match run(cmd).and_then(|out| parse_count(&out)) {
                    Ok(observed) if observed == expected => ClaimCheckOutcome::Match,
                    Ok(observed) => ClaimCheckOutcome::Mismatch { observed },
                    Err(why) => ClaimCheckOutcome::Unverifiable(why),
                },
            };
            ClaimCheck {
                id: claim.id.clone(),
                expected: claim.numeric_value,
                outcome,
            }
        })
        .collect()
}
