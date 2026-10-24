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
        _available_models: vec![],
        _context_turns: None,
        _max_tokens: None,
        _temperature: None,
        _top_p: None,
        _reasoning_profile: "none".into(),
        _context_source: "unit-test".into(),
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
