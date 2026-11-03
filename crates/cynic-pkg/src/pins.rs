//! Ecosystem pins loader and models.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PinsFile {
    pub pins: Vec<PinEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PinEntry {
    pub id: String,
    #[serde(default)]
    pub min_version: Option<String>,
    #[serde(default)]
    pub min_build: Option<u64>,
    #[serde(default)]
    pub desired_build: Option<u64>,
    #[serde(default)]
    pub min_capability: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub notes: Option<String>,
}

pub fn load_pins() -> (PathBuf, Vec<PinEntry>) {
    let path = resolve_pins_path();
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return (path, Vec::new());
    };
    match serde_json::from_str::<PinsFile>(&raw) {
        Ok(file) => (path, file.pins),
        Err(_) => (path, Vec::new()),
    }
}

pub fn resolve_pins_path() -> PathBuf {
    if let Ok(path) = std::env::var("CYNIC_ECOSYSTEM_PINS") {
        return PathBuf::from(path);
    }

    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..8 {
        let candidate = dir.join("infra/ecosystem-pins.json");
        if candidate.is_file() {
            return candidate;
        }
        if !dir.pop() {
            break;
        }
    }
    PathBuf::from("infra/ecosystem-pins.json")
}
