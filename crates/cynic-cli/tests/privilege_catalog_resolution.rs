use std::fs;
use std::process::Command;

#[test]
fn privilege_catalog_honors_cynic_catalog_env() {
    let root = std::env::temp_dir().join(format!(
        "cynic-cli-privilege-catalog-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let outside = root.join("outside");
    let config = root.join("etc/cynic");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(&config).unwrap();
    let catalog = root.join("catalog.json");
    fs::write(
        &catalog,
        include_str!("../../../infra/capabilities/catalog.json"),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cynic"))
        .current_dir(&outside)
        .env("CYNIC_CATALOG", &catalog)
        .args(["privilege", "catalog", "--json"])
        .output()
        .expect("run cynic");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["catalog_path"], catalog.to_string_lossy().to_string());
}

#[test]
fn privilege_catalog_defaults_to_installed_system_catalog() {
    let outside = std::env::temp_dir().join(format!(
        "cynic-cli-privilege-catalog-default-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&outside).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cynic"))
        .current_dir(&outside)
        .env_remove("CYNIC_CATALOG")
        .env_remove("CYNIC_CAPABILITY_CATALOG")
        .args(["privilege", "catalog", "--json"])
        .output()
        .expect("run cynic");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["status"], "ok");
    assert_eq!(
        payload["catalog_path"],
        "/etc/cynic/capabilities.installed.json"
    );
}
