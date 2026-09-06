use super::*;
use std::os::unix::fs::{MetadataExt, symlink};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture() -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "surface-registry-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("fragments/nested")).unwrap();
    fs::write(
        root.join("fragments/main.json"),
        r#"{"id":"main","value":1}"#,
    )
    .unwrap();
    fs::write(
        root.join("manifest.json"),
        format!(
            r#"{{"schema":"{SCHEMA}","surfaces":[{{"id":"main","path":"fragments/main.json"}}]}}"#
        ),
    )
    .unwrap();
    (root.join("manifest.json"), root.join("out.json"))
}

fn cleanup(manifest: &Path) {
    let _ = fs::remove_dir_all(manifest.parent().unwrap());
}

fn assert_err(manifest: &Path, needle: &str) {
    let error = assemble(manifest).unwrap_err();
    assert!(error.contains(needle), "{error}");
}

#[test]
fn positive_assembly_and_deterministic_output() {
    let (manifest, output) = fixture();
    write(&manifest, &output).unwrap();
    check(&manifest, &output).unwrap();
    let first = fs::read_to_string(&output).unwrap();
    let first_inode = fs::metadata(&output).unwrap().ino();
    write(&manifest, &output).unwrap();
    assert_eq!(first, fs::read_to_string(&output).unwrap());
    assert_ne!(first_inode, fs::metadata(&output).unwrap().ino());
    assert!(
        !fs::read_dir(output.parent().unwrap())
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("cynic-tmp"))
    );
    cleanup(&manifest);
}

#[test]
fn descriptor_identity_mismatch_is_rejected_before_reading() {
    let (manifest, _) = fixture();
    let root = manifest.parent().unwrap();
    let root_file = open_verified_directory(root, "repository root").unwrap();
    let relative = Path::new("fragments/main.json");
    let first = open_relative(&root_file, relative, "main").unwrap();
    let first_identity = FileIdentity::from_metadata(&first.metadata().unwrap());

    let replacement = root.join("fragments/replacement.json");
    fs::write(&replacement, r#"{"id":"main","value":2}"#).unwrap();
    fs::rename(&replacement, root.join(relative)).unwrap();

    let second = open_relative(&root_file, relative, "main").unwrap();
    let error = read_fragment(
        FragmentDescriptor {
            relative: relative.to_path_buf(),
            file: second,
            identity: first_identity,
        },
        "main",
    )
    .unwrap_err();
    assert!(error.contains("identity mismatch"), "{error}");
    cleanup(&manifest);
}

#[test]
fn canonical_repository_fixture_assembles() {
    let manifest =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../infra/surface-registry/manifest.json");
    let document = assemble(&manifest).expect("canonical surface registry fixture");
    assert_eq!(document["schema"], SCHEMA);
    assert!(!document["surfaces"].as_array().unwrap().is_empty());
}

#[test]
fn duplicate_active_ids_are_rejected() {
    let (manifest, _) = fixture();
    let root = manifest.parent().unwrap();
    fs::write(root.join("fragments/second.json"), r#"{"id":"main"}"#).unwrap();
    fs::write(
        &manifest,
        format!(
            r#"{{"schema":"{SCHEMA}","surfaces":[{{"id":"main","path":"fragments/main.json"}},{{"id":"main","path":"fragments/second.json"}}]}}"#
        ),
    )
    .unwrap();
    assert_err(&manifest, "duplicate surface id");
    cleanup(&manifest);
}

#[test]
fn duplicate_fragment_paths_are_rejected() {
    let (manifest, _) = fixture();
    fs::write(
        &manifest,
        format!(
            r#"{{"schema":"{SCHEMA}","surfaces":[{{"id":"main","path":"fragments/main.json"}},{{"id":"other","path":"fragments/main.json"}}]}}"#
        ),
    )
    .unwrap();
    assert_err(&manifest, "duplicate fragment path");
    cleanup(&manifest);
}

#[test]
fn active_retired_overlap_and_missing_reason_are_rejected() {
    let (manifest, _) = fixture();
    let root = manifest.parent().unwrap();
    fs::write(root.join("fragments/old.json"), r#"{"id":"old"}"#).unwrap();
    fs::write(
        &manifest,
        format!(
            r#"{{"schema":"{SCHEMA}","surfaces":[{{"id":"main","path":"fragments/main.json"}}],"retired":[{{"id":"main","reason":"legacy","path":"fragments/main.json"}}]}}"#
        ),
    )
    .unwrap();
    assert_err(&manifest, "retired surface is also active");
    fs::write(
        &manifest,
        format!(
            r#"{{"schema":"{SCHEMA}","surfaces":[{{"id":"main","path":"fragments/main.json"}}],"retired":[{{"id":"old","path":"fragments/old.json"}}]}}"#
        ),
    )
    .unwrap();
    assert_err(&manifest, "missing reason");
    cleanup(&manifest);
}

#[test]
fn path_escape_and_outside_paths_are_rejected() {
    let (manifest, _) = fixture();
    let root = manifest.parent().unwrap();
    fs::write(root.join("outside.json"), r#"{"id":"main"}"#).unwrap();
    fs::write(
        &manifest,
        format!(r#"{{"schema":"{SCHEMA}","surfaces":[{{"id":"main","path":"outside.json"}}]}}"#),
    )
    .unwrap();
    assert_err(&manifest, "beneath fragments directory");
    fs::write(
        &manifest,
        format!(r#"{{"schema":"{SCHEMA}","surfaces":[{{"id":"main","path":"../outside.json"}}]}}"#),
    )
    .unwrap();
    assert_err(&manifest, "safe and relative");
    cleanup(&manifest);
}

#[test]
fn symlinks_and_nested_orphans_are_rejected() {
    let (manifest, _) = fixture();
    let root = manifest.parent().unwrap();
    symlink(
        root.join("fragments/main.json"),
        root.join("fragments/link.json"),
    )
    .unwrap();
    assert_err(&manifest, "symlink");
    fs::remove_file(root.join("fragments/link.json")).unwrap();
    fs::write(
        root.join("fragments/nested/orphan.json"),
        r#"{"id":"orphan"}"#,
    )
    .unwrap();
    assert_err(&manifest, "nested/orphan.json");
    cleanup(&manifest);
}

#[test]
fn fifo_is_rejected_without_blocking() {
    let (manifest, _) = fixture();
    let fifo = manifest.parent().unwrap().join("fragments/fifo");
    std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap();
    assert_err(&manifest, "special entry");
    cleanup(&manifest);
}

#[test]
fn manifest_fifo_is_rejected_without_blocking() {
    let (manifest, _) = fixture();
    fs::remove_file(&manifest).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&manifest)
            .status()
            .unwrap()
            .success()
    );
    assert_err(&manifest, "manifest is not a regular file");
    cleanup(&manifest);
}

#[test]
fn output_symlink_is_rejected_without_touching_target() {
    let (manifest, output) = fixture();
    let target = manifest.parent().unwrap().join("protected.json");
    fs::write(&target, "sentinel").unwrap();
    symlink(&target, &output).unwrap();
    let error = write(&manifest, &output).unwrap_err();
    assert!(error.contains("symlink"), "{error}");
    assert_eq!(fs::read_to_string(target).unwrap(), "sentinel");
    cleanup(&manifest);
}

#[test]
fn output_parent_symlink_is_rejected_without_creating_directories() {
    let (manifest, output) = fixture();
    let root = manifest.parent().unwrap();
    let real_parent = root.join("real-output");
    let linked_parent = root.join("linked-output");
    fs::create_dir(&real_parent).unwrap();
    symlink(&real_parent, &linked_parent).unwrap();
    let linked_output = linked_parent.join("out.json");
    let error = write(&manifest, &linked_output).unwrap_err();
    assert!(
        error.contains("output parent") && error.contains("unsafe"),
        "{error}"
    );
    assert!(!real_parent.join("out.json").exists());
    assert!(!real_parent.read_dir().unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("cynic-tmp")
    }));
    cleanup(&manifest);
}

#[test]
fn output_directory_is_rejected_without_blocking() {
    let (manifest, output) = fixture();
    fs::create_dir(&output).unwrap();
    let error = write(&manifest, &output).unwrap_err();
    assert!(error.contains("not a regular file"), "{error}");
    cleanup(&manifest);
}

#[test]
fn output_fifo_is_rejected_without_blocking() {
    let (manifest, output) = fixture();
    std::process::Command::new("mkfifo")
        .arg(&output)
        .status()
        .unwrap();
    let error = write(&manifest, &output).unwrap_err();
    assert!(error.contains("not a regular file"), "{error}");
    cleanup(&manifest);
}

#[test]
fn duplicate_retired_ids_are_rejected() {
    let (manifest, _) = fixture();
    let root = manifest.parent().unwrap();
    fs::write(root.join("fragments/old-a.json"), r#"{"id":"old"}"#).unwrap();
    fs::write(root.join("fragments/old-b.json"), r#"{"id":"old"}"#).unwrap();
    fs::write(
        &manifest,
        format!(
            r#"{{"schema":"{SCHEMA}","surfaces":[{{"id":"main","path":"fragments/main.json"}}],"retired":[{{"id":"old","reason":"legacy","path":"fragments/old-a.json"}},{{"id":"old","reason":"legacy","path":"fragments/old-b.json"}}]}}"#
        ),
    )
    .unwrap();
    assert_err(&manifest, "duplicate retired");
    cleanup(&manifest);
}
