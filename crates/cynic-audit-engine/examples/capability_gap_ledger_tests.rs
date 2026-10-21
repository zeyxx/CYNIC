use super::*;

#[test]
fn extracts_literal_vs_dynamic() {
    assert_eq!(
        extract_program("let x = Command::new(\"git\").arg(\"status\");"),
        Some("git".to_string())
    );
    assert_eq!(
        extract_program("Command::new(program)"),
        Some("dynamic".to_string())
    );
    assert_eq!(
        extract_program("std::process::Command::new(env!(\"CARGO_BIN_EXE_cynic\"))"),
        Some("dynamic".to_string())
    );
}

#[test]
fn classifies_risk() {
    assert_eq!(classify("ssh", "x.rs"), "remote-exec");
    assert_eq!(classify("dynamic", "x.rs"), "self-exec");
    assert_eq!(classify("git", "x.rs"), "local-build");
    assert_eq!(classify("cargo", "x.rs"), "package-manager");
    assert_eq!(classify("curl", "x.rs"), "provider-specific");
    assert_eq!(classify("which", "x.rs"), "observability");
}

#[test]
fn sorts_deterministically() {
    let mut surfaces = vec![
        Surface {
            path: "b.rs".to_string(),
            line: 2,
            program: "x".to_string(),
            risk_class: "local-build".to_string(),
            is_capability_binary: false,
        },
        Surface {
            path: "a.rs".to_string(),
            line: 5,
            program: "y".to_string(),
            risk_class: "local-build".to_string(),
            is_capability_binary: false,
        },
        Surface {
            path: "a.rs".to_string(),
            line: 1,
            program: "z".to_string(),
            risk_class: "local-build".to_string(),
            is_capability_binary: false,
        },
    ];
    sort_surfaces(&mut surfaces);
    assert_eq!(surfaces[0].path, "a.rs");
    assert_eq!(surfaces[0].line, 1);
    assert_eq!(surfaces[1].line, 5);
    assert_eq!(surfaces[2].path, "b.rs");
}

#[test]
fn a_test_module_is_not_a_production_surface() {
    let lines: Vec<&str> = vec![
        "fn production() {",
        "    let _ = Command::new(\"git\");",
        "}",
        "#[cfg(test)]",
        "mod tests {",
        "    fn helper() { let _ = Command::new(\"git\"); }",
        "}",
    ];
    assert!(!is_test_surface("crates/x/src/a.rs", &lines, 1));
    assert!(is_test_surface("crates/x/src/a.rs", &lines, 5));
}

#[test]
fn a_standalone_test_file_is_not_a_production_surface() {
    let lines = vec!["fn helper() { let _ = Command::new(\"git\"); }"];
    assert!(is_test_surface(
        "crates/x/src/policy_tests.rs",
        &lines,
        0
    ));
}

#[test]
fn an_integration_test_file_is_not_a_production_surface() {
    let lines: Vec<&str> = vec!["fn helper() { let _ = Command::new(\"git\"); }"];
    assert!(is_test_surface("crates/x/tests/a.rs", &lines, 0));
}

#[test]
fn relative_path_to_workspace() {
    let root = Path::new("/tmp/ws");
    let path = Path::new("/tmp/ws/crates/foo.rs");
    assert_eq!(relative_to_workspace(root, path), "crates/foo.rs");
    let outside = Path::new("/elsewhere/bar.rs");
    assert_eq!(relative_to_workspace(root, outside), "/elsewhere/bar.rs");
}
