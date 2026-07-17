//! Phase promotion (pure state transition + persistence).

use std::path::Path;

use super::state_store::{load_state, save_state};
use super::time::now_ms;
use super::types::{
    LifecyclePhase, LifecyclePromoteReport, UPDATE_LIFECYCLE_PROMOTE_SCHEMA,
};

pub fn promote(
    state_path: &Path,
    to: Option<&str>,
    yes: bool,
) -> Result<LifecyclePromoteReport, String> {
    let mut state = load_state(state_path);
    let from = state.current_phase.clone();
    let target = if let Some(t) = to {
        LifecyclePhase::parse(t).ok_or_else(|| format!("unknown phase: {t}"))?
    } else {
        from.next()
            .ok_or_else(|| "already at production".to_string())?
    };

    if target.index() != from.index() + 1 {
        return Ok(LifecyclePromoteReport {
            schema: UPDATE_LIFECYCLE_PROMOTE_SCHEMA,
            status: "blocked",
            from: from.as_str(),
            to: Some(target.as_str()),
            mutates: false,
            message: format!(
                "must promote one phase at a time ({} → {:?}), not jump to {}",
                from.as_str(),
                from.next().map(|p| p.as_str()),
                target.as_str()
            ),
            state,
        });
    }

    if state.last_gate_ok != Some(true) {
        return Ok(LifecyclePromoteReport {
            schema: UPDATE_LIFECYCLE_PROMOTE_SCHEMA,
            status: "blocked",
            from: from.as_str(),
            to: Some(target.as_str()),
            mutates: false,
            message: "gate not green — run: cynic lifecycle gate --yes".into(),
            state,
        });
    }

    if !yes {
        return Ok(LifecyclePromoteReport {
            schema: UPDATE_LIFECYCLE_PROMOTE_SCHEMA,
            status: "planned",
            from: from.as_str(),
            to: Some(target.as_str()),
            mutates: false,
            message: format!(
                "would promote {} → {} — re-run with --yes",
                from.as_str(),
                target.as_str()
            ),
            state,
        });
    }

    state.current_phase = target.clone();
    state.last_promote_at_unix_ms = Some(now_ms());
    state.last_gate_ok = None;
    state.updated_at_unix_ms = now_ms();
    state.notes.push(format!(
        "promoted {} → {} at {}",
        from.as_str(),
        target.as_str(),
        state.updated_at_unix_ms
    ));
    save_state(state_path, &state)?;

    Ok(LifecyclePromoteReport {
        schema: UPDATE_LIFECYCLE_PROMOTE_SCHEMA,
        status: "ok",
        from: from.as_str(),
        to: Some(target.as_str()),
        mutates: true,
        message: format!(
            "promoted to {} — run gates for new phase before next promote",
            target.as_str()
        ),
        state,
    })
}
