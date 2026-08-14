use std::env;
use std::sync::Arc;

use cynic_inference::{FsChatSessionStore, PathAuthority, PrincipalPaths};

use super::handlers::blocked_not_ready;
use super::runtime::ChatRuntime;

#[cfg(test)]
#[test]
fn blocked_not_ready_report_is_fail_closed() {
    let store = match FsChatSessionStore::new(env::temp_dir().join("cynic-chat-test-store")) {
        Ok(s) => Arc::new(s),
        Err(_) => return, // skip if temp unavailable
    };
    let runtime = ChatRuntime {
        store,
        service: None,
        ready: false,
        use_mock: false,
        endpoint: None,
        model: None,
        source: "none".into(),
        detail: "no endpoint".into(),
        session_root: "/tmp/x".into(),
        available_models: vec![],
        context_turns: None,
        max_tokens: None,
        temperature: None,
        top_p: None,
        reasoning_profile: "none".into(),
        context_source: "unit-test".into(),
    };
    let paths = PrincipalPaths::from_home(
        PathAuthority::RuntimePrincipal,
        "cynic".into(),
        std::path::PathBuf::from("/var/lib/cynic"),
        vec![],
    );
    let r = blocked_not_ready(&runtime, &paths);
    assert_eq!(r.status, "blocked");
    assert!(!r.ready);
    assert!(r.content.is_none());
    assert!(r.blocked_reason.is_some());
}
