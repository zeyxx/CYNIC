//! Application layer: normalize a harness source into the canonical store.
//!
//! Strict port of `.../usecases/normalize.py`. Imports domain + ports,
//! never adapters directly — the adapter is injected through the port.

use std::collections::HashMap;
use std::time::Instant;

use serde::Serialize;

use crate::ports::{CanonicalStorePort, NormalizerPort};
use crate::session::NormalizedSession;

#[derive(Debug, Clone, Default, Serialize)]
pub struct NormalizeReport {
    pub harness: String,
    pub sessions_ok: usize,
    pub sessions_failed: usize,
    pub totals: HashMap<String, usize>,
    pub failures: Vec<(String, String)>,
}

impl NormalizeReport {
    pub fn add(&mut self, stats: &crate::session::SessionStats) {
        for (k, v) in [
            ("turns", stats.turns),
            ("tools", stats.tools),
            ("reasoning", stats.reasoning),
            ("edges", stats.edges),
            ("states", stats.states),
            ("human", stats.human),
        ] {
            *self.totals.entry(k.to_owned()).or_insert(0) += v;
        }
    }
}

fn emit(kind: &str, fields: serde_json::Value) {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "schema".into(),
        serde_json::Value::from("cynic.canonical.event.v1"),
    );
    obj.insert("event".into(), serde_json::Value::from(kind));
    if let serde_json::Value::Object(extra) = fields {
        obj.extend(extra);
    }
    println!("{}", serde_json::Value::Object(obj));
}

fn event_fields(pairs: &[(&str, serde_json::Value)]) -> serde_json::Value {
    serde_json::Value::Object(
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect(),
    )
}

/// Normalize every session the adapter's bound source contains.
///
/// The adapter enumerates its own sessions (`session_refs`); the application
/// layer never passes ids it could not have known how to build.
pub fn normalize_source<N, S>(
    normalizer: &N,
    store: &S,
    progress_every: usize,
) -> NormalizeReport
where
    N: NormalizerPort,
    S: CanonicalStorePort,
{
    let mut report = NormalizeReport {
        harness: normalizer.harness().to_owned(),
        ..Default::default()
    };
    let t0 = Instant::now();
    let session_refs = normalizer.session_refs();
    emit(
        "start",
        event_fields(&[
            ("harness", serde_json::Value::from(normalizer.harness())),
            (
                "sessions",
                serde_json::Value::from(session_refs.len() as u64),
            ),
        ]),
    );
    if session_refs.is_empty() {
        emit(
            "empty",
            event_fields(&[
                ("harness", serde_json::Value::from(normalizer.harness())),
                (
                    "reason",
                    serde_json::Value::from("no sessions found in bound source"),
                ),
            ]),
        );
        return report;
    }

    for (i, session_ref) in session_refs.iter().enumerate() {
        let n = i + 1;
        let outcome: Result<NormalizedSession, String> = normalizer
            .normalize(session_ref)
            .and_then(|s| s.check().map(|()| s).map_err(|e| e.to_string()))
            .and_then(|s| {
                store
                    .write(&s)
                    .map(|()| s)
                    .map_err(|e| e.to_string())
            });
        match outcome {
            Ok(session) => {
                report.sessions_ok += 1;
                report.add(&session.stats());
            }
            Err(exc) => {
                report.sessions_failed += 1;
                report.failures.push((session_ref.clone(), exc.clone()));
                emit(
                    "session_failed",
                    event_fields(&[
                        ("session_ref", serde_json::Value::from(session_ref.clone())),
                        ("error", serde_json::Value::from(exc)),
                    ]),
                );
            }
        }
        if progress_every > 0 && n % progress_every == 0 {
            emit(
                "progress",
                event_fields(&[
                    ("done", serde_json::Value::from(n as u64)),
                    ("total", serde_json::Value::from(session_refs.len() as u64)),
                    ("ok", serde_json::Value::from(report.sessions_ok as u64)),
                    (
                        "failed",
                        serde_json::Value::from(report.sessions_failed as u64),
                    ),
                    (
                        "elapsed_s",
                        serde_json::Value::from(t0.elapsed().as_secs_f64().round() as u64),
                    ),
                ]),
            );
        }
    }
    emit(
        "done",
        event_fields(&[
            ("harness", serde_json::Value::from(normalizer.harness())),
            ("ok", serde_json::Value::from(report.sessions_ok as u64)),
            (
                "failed",
                serde_json::Value::from(report.sessions_failed as u64),
            ),
            (
                "elapsed_s",
                serde_json::Value::from(t0.elapsed().as_secs_f64().round() as u64),
            ),
            ("totals", serde_json::to_value(&report.totals).unwrap_or_default()),
        ]),
    );
    report
}

/// Normalize a single session. Used by tests and one-off inspections.
pub fn normalize_session<N, S>(
    normalizer: &N,
    store: &S,
    session_ref: &str,
) -> NormalizeReport
where
    N: NormalizerPort,
    S: CanonicalStorePort,
{
    let mut report = NormalizeReport {
        harness: normalizer.harness().to_owned(),
        ..Default::default()
    };
    let outcome: Result<NormalizedSession, String> = normalizer
        .normalize(session_ref)
        .and_then(|s| s.check().map(|()| s).map_err(|e| e.to_string()))
        .and_then(|s| {
            store
                .write(&s)
                .map(|()| s)
                .map_err(|e| e.to_string())
        });
    match outcome {
        Ok(session) => {
            report.sessions_ok = 1;
            report.add(&session.stats());
        }
        Err(exc) => {
            report.sessions_failed = 1;
            report.failures.push((session_ref.to_owned(), exc));
        }
    }
    report
}
