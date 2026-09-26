use super::collect::normalize_version;

#[test]
fn extract_version_handles_common_formats() {
    assert_eq!(normalize_version("v1.82.1"), "1.82.1");
    assert_eq!(normalize_version("b9253"), "9253");
    assert_eq!(
        normalize_version("surrealdb 3.0.3 for linux on x86_64"),
        "3.0.3"
    );
    assert_eq!(normalize_version("1.82.1"), "1.82.1");
    assert_eq!(normalize_version("Version 0.7.7"), "0.7.7");
}

#[test]
fn kernel_version_is_available() {
    let version = env!("CARGO_PKG_VERSION");
    assert!(!version.is_empty());
}
