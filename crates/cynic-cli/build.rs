use std::{
    env, fs,
    path::{Path, PathBuf},
    process::exit,
};

use serde_json::Value;
use std::collections::BTreeSet;

const SURFACE_REGISTRY_RELATIVE_MANIFEST: &str = "../../infra/surface-registry/manifest.json";
const GENERATED_FILE: &str = "surface_registry_fragment_index.rs";
const CYNIC_SURFACE_REGISTRY_SCHEMA: &str = "cynic.surface_registry.v1";

fn fail(message: &str) -> ! {
    eprintln!("surface registry fragment index generation failed: {message}");
    exit(1);
}

fn manifest_path(cargo_manifest_dir: &Path) -> PathBuf {
    let path = cargo_manifest_dir.join(SURFACE_REGISTRY_RELATIVE_MANIFEST);
    if !path.exists() {
        fail(&format!("manifest not found at {}", path.display()));
    }
    path
}

fn parse_manifest(path: &Path) -> Value {
    let payload = fs::read_to_string(path).unwrap_or_else(|error| {
        fail(&format!("cannot read manifest {}: {error}", path.display()));
    });
    serde_json::from_str(&payload).unwrap_or_else(|error| {
        fail(&format!(
            "invalid JSON manifest {}: {error}",
            path.display()
        ));
    })
}

fn normalize_path(path: &str) -> &str {
    if path.ends_with(".json")
        && !path.contains("../")
        && !path.starts_with('/')
        && !path.contains('\\')
    {
        path
    } else {
        fail(&format!("invalid fragment path: {path}"));
    }
}

fn write_index(manifest: &Value) {
    let schema = manifest
        .get("schema")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if schema != CYNIC_SURFACE_REGISTRY_SCHEMA {
        fail(&format!(
            "manifest schema mismatch: expected {CYNIC_SURFACE_REGISTRY_SCHEMA}, got {schema}"
        ));
    }

    let surfaces = manifest
        .get("surfaces")
        .and_then(Value::as_array)
        .unwrap_or_else(|| {
            fail("manifest.surfaces must be a JSON array");
        });
    if surfaces.is_empty() {
        fail("manifest.surfaces must not be empty");
    }

    let fragments_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("infra")
        .join("surface-registry");
    let mut seen_ids = BTreeSet::new();

    let mut out = String::new();
    out.push_str(&format!(
        "pub(crate) const SURFACE_REGISTRY_FRAGMENT_INDEX: [(&str, &str); {}] = [\n",
        surfaces.len()
    ));
    for surface in surfaces {
        let id = surface
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                fail("manifest surface is missing id");
            });
        if !seen_ids.insert(id.to_string()) {
            fail(&format!("duplicate surface id in manifest: {id}"));
        }
        let path = surface
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                fail(&format!("manifest surface {id} is missing path"));
            });
        let path = normalize_path(path);
        let fragment_path = fragments_root.join(path);
        println!("cargo:rerun-if-changed=../../infra/surface-registry/{path}");
        if !fragment_path.exists() {
            fail(&format!(
                "manifest references missing fragment path {path} for surface {id}"
            ));
        }
        let fragment_payload = fs::read_to_string(&fragment_path).unwrap_or_else(|error| {
            fail(&format!("cannot read fragment {path}: {error}"));
        });
        let fragment: Value = serde_json::from_str(&fragment_payload).unwrap_or_else(|error| {
            fail(&format!("invalid JSON in fragment {path}: {error}"));
        });
        let fragment_id = fragment
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                fail(&format!("fragment {path} is missing an id"));
            });
        if fragment_id != id {
            fail(&format!(
                "fragment {path} id mismatch: manifest expects {id}, fragment has {fragment_id}"
            ));
        }
        out.push_str("    ");
        out.push_str(&format!(
            "({id:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../infra/surface-registry/{path}\"))),\n"
        ));
        out.push('\n');
    }
    out.push_str("];\n");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap_or_else(|_| {
        fail("OUT_DIR is not set");
    }))
    .join(GENERATED_FILE);
    fs::write(&out_path, out).unwrap_or_else(|error| {
        fail(&format!("unable to write {}: {error}", out_path.display()));
    });
}

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    let git_sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let git_sha = if git_sha.is_empty() {
        "unknown".to_string()
    } else {
        git_sha
    };
    let profile = env::var("PROFILE").unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=CYNIC_GIT_SHA={git_sha}");
    println!("cargo:rustc-env=CYNIC_BUILD_PROFILE={profile}");

    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| {
        fail("CARGO_MANIFEST_DIR is not set");
    });
    let manifest_path = manifest_path(Path::new(&cargo_manifest_dir));
    println!(
        "cargo:rerun-if-changed={}",
        SURFACE_REGISTRY_RELATIVE_MANIFEST
    );
    let manifest = parse_manifest(&manifest_path);
    write_index(&manifest);
}
