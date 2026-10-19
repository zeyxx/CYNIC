use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("contracts").exists() {
        if !dir.pop() {
            panic!("Could not find repo root containing contracts/");
        }
    }
    dir
}

#[test]
fn test_inference_calibration_contract_integrity() {
    let root = repo_root();
    let contract_path = root.join("contracts/inference-calibration.json");
    assert!(
        contract_path.is_file(),
        "contracts/inference-calibration.json must exist"
    );

    let content = fs::read_to_string(&contract_path).expect("read contract");
    let json: serde_json::Value = serde_json::from_str(&content).expect("parse json");

    assert_eq!(json["schema"], "cynic.inference.calibration.v1");
    assert_eq!(json["status"], "normative");

    // Substrate integrity
    assert_eq!(json["hardware_substrate"]["node_name"], "cortex");
    assert_eq!(json["hardware_substrate"]["swap_bytes"], 0);
    assert_eq!(
        json["hardware_substrate"]["bandwidth_tier"],
        "sub100_legacy_ddr"
    );

    // Backends verification
    let backends = json["calibrated_backends"]
        .as_array()
        .expect("backends array");
    assert!(
        !backends.is_empty(),
        "calibrated backends must not be empty"
    );

    let lfm = backends
        .iter()
        .find(|b| b["alias"] == "lfm25-2b-q4")
        .expect("lfm backend");
    assert_eq!(lfm["port"], 8085);
    assert_eq!(lfm["architecture"], "lfm2");
    assert_eq!(lfm["streaming_sse"], true);

    let qwen = backends
        .iter()
        .find(|b| b["alias"] == "qwen35-2b-q4")
        .expect("qwen backend");
    assert_eq!(qwen["port"], 8088);
    assert_eq!(qwen["architecture"], "qwen2");
    assert_eq!(qwen["streaming_sse"], true);

    // Admission policy bounds
    assert_eq!(json["admission_policy"]["concurrent_active_models_max"], 1);
    assert_eq!(json["admission_policy"]["safety_margin_ratio"], 0.20);
}
