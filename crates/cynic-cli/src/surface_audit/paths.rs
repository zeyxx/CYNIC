use std::path::{Path, PathBuf};

pub(crate) fn read_adapter_source(path: &Path) -> Result<String, std::io::Error> {
    let mut source = std::fs::read_to_string(path)?;
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return Ok(source);
    };
    let Some(parent) = path.parent() else {
        return Ok(source);
    };
    let module_dir = parent.join(stem);
    if module_dir.is_dir() {
        for child in rust_files_under(&module_dir)? {
            source.push('\n');
            source.push_str(&std::fs::read_to_string(child)?);
        }
    }
    let sibling_prefix = format!("{stem}_");
    let mut sibling_modules = std::fs::read_dir(parent)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|candidate| {
            candidate.extension().and_then(|value| value.to_str()) == Some("rs")
                && candidate
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .is_some_and(|candidate_stem| candidate_stem.starts_with(&sibling_prefix))
        })
        .collect::<Vec<_>>();
    sibling_modules.sort();
    for sibling in sibling_modules {
        source.push('\n');
        source.push_str(&std::fs::read_to_string(sibling)?);
    }
    Ok(source)
}

pub(crate) fn adapter_module_path(owner_crate: &str, adapter_module: &str) -> PathBuf {
    if let Some((crate_name, module_name)) = adapter_module.split_once("::") {
        if matches!(
            crate_name,
            "cynic-host" | "cynic-node" | "cynic-protocol" | "cynic-cli" | "cynic-inference"
        ) {
            return crate_module_path(crate_name, module_name);
        }
    }
    if owner_crate == "cynic-cli" {
        return crate_module_path(owner_crate, adapter_module);
    }
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let parts = adapter_module.split("::").collect::<Vec<_>>();
    match parts.as_slice() {
        [module] => path.push(format!("{module}.rs")),
        [module, child] => {
            let module_file = path.join(format!("{module}.rs"));
            let nested_file = path.join(module).join(format!("{child}.rs"));
            if nested_file.exists() || !module_file.exists() {
                path = nested_file;
            } else {
                path = module_file;
            }
        }
        _ => {
            for part in &parts[..parts.len().saturating_sub(1)] {
                path.push(part);
            }
            if let Some(last) = parts.last() {
                path.push(format!("{last}.rs"));
            }
        }
    }
    path
}

pub(crate) fn owner_module_path(owner_crate: &str, module: &str) -> PathBuf {
    if let Some((crate_name, module_name)) = module.split_once("::") {
        if matches!(
            crate_name,
            "cynic-host" | "cynic-node" | "cynic-protocol" | "cynic-cli" | "cynic-inference"
        ) {
            return crate_module_path(crate_name, module_name);
        }
    }
    crate_module_path(owner_crate, module)
}

fn crate_module_path(crate_name: &str, module: &str) -> PathBuf {
    let crate_dir = match crate_name {
        "cynic-cli" => env!("CARGO_MANIFEST_DIR").to_string(),
        "cynic-host" => format!("{}/../cynic-host", env!("CARGO_MANIFEST_DIR")),
        "cynic-node" => format!("{}/../cynic-node", env!("CARGO_MANIFEST_DIR")),
        "cynic-protocol" => format!("{}/../cynic-protocol", env!("CARGO_MANIFEST_DIR")),
        "cynic-inference" => format!("{}/../cynic-inference", env!("CARGO_MANIFEST_DIR")),
        _ => env!("CARGO_MANIFEST_DIR").to_string(),
    };
    let mut path = PathBuf::from(crate_dir).join("src");
    let parts = module.split("::").collect::<Vec<_>>();
    match parts.as_slice() {
        [single] => {
            let file = path.join(format!("{single}.rs"));
            let mod_file = path.join(single).join("mod.rs");
            if mod_file.exists() { mod_file } else { file }
        }
        _ => {
            for part in &parts[..parts.len().saturating_sub(1)] {
                path.push(part);
            }
            if let Some(last) = parts.last() {
                let file = path.join(format!("{last}.rs"));
                let mod_file = path.join(last).join("mod.rs");
                if mod_file.exists() {
                    return mod_file;
                }
                return file;
            }
            path
        }
    }
}

fn rust_files_under(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(current)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}
