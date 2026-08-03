#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BootstrapRuntimeArtifact {
    pub binary: String,
    pub install_path: String,
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn artifact_commands(artifacts: &[BootstrapRuntimeArtifact], command: &str) -> String {
    artifacts
        .iter()
        .map(|artifact| {
            format!(
                "{command} {} {}\n",
                shell_quote(&artifact.binary),
                shell_quote(&artifact.install_path)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

pub(super) fn install_preflight_fragment(artifacts: &[BootstrapRuntimeArtifact]) -> String {
    let commands = artifact_commands(artifacts, "preflight_runtime_artifact");
    format!(
        r#"seed_digest_line="$(sha256sum "$ROOT/seed-manifest.json")"
seed_digest="${{seed_digest_line%% *}}"
desired_release="releases/bootstrap-$seed_digest"

preflight_runtime_artifact() {{
  binary="$1"
  entrypoint="$2"
  test -x "$ROOT/bin/$binary" || {{
    echo "runtime artifact is unavailable: $binary" >&2
    exit 1
  }}
  test ! -d "$entrypoint" || {{
    echo "runtime entrypoint is an unsafe directory: $entrypoint" >&2
    exit 1
  }}
}}

{commands}if test -e "$RUNTIME_INSTALL_ROOT/current" || test -L "$RUNTIME_INSTALL_ROOT/current"; then
  test -L "$RUNTIME_INSTALL_ROOT/current" || {{
    echo "runtime current projection is not a symlink" >&2
    exit 1
  }}
  test "$(readlink "$RUNTIME_INSTALL_ROOT/current")" = "$desired_release" || {{
    echo "another runtime seed is already active; use the signed release activation surface" >&2
    exit 1
  }}
  marker="$RUNTIME_INSTALL_ROOT/current/.cynic-runtime-release.json"
  test -f "$marker"
  grep -F "\"basket_digest\": \"sha256:$seed_digest\"" "$marker" >/dev/null || {{
    echo "active runtime seed marker does not match the requested seed" >&2
    exit 1
  }}
fi
"#
    )
}

pub(super) fn install_activation_fragment(artifacts: &[BootstrapRuntimeArtifact]) -> String {
    if artifacts.is_empty() {
        return String::new();
    }
    let stage = artifact_commands(artifacts, "stage_runtime_artifact");
    let prepare = artifact_commands(artifacts, "prepare_runtime_entrypoint");
    let activate = artifact_commands(artifacts, "activate_runtime_entrypoint");
    let read_back = artifact_commands(artifacts, "read_back_runtime_entrypoint");
    format!(
        r#"stage_runtime_artifact() {{
  binary="$1"
  install -o root -g root -m 0755 "$ROOT/bin/$binary" "$BOOTSTRAP_RELEASE/$binary"
}}

prepare_runtime_entrypoint() {{
  binary="$1"
  entrypoint="$2"
  install -d -o root -g root -m 0755 "$(dirname "$entrypoint")"
  next_entrypoint="$(dirname "$entrypoint")/.cynic-${{binary}}.next.$$"
  test ! -e "$next_entrypoint" && test ! -L "$next_entrypoint"
  ln -s "$RUNTIME_INSTALL_ROOT/current/$binary" "$next_entrypoint"
}}

activate_runtime_entrypoint() {{
  binary="$1"
  entrypoint="$2"
  next_entrypoint="$(dirname "$entrypoint")/.cynic-${{binary}}.next.$$"
  mv "$next_entrypoint" "$entrypoint"
}}

read_back_runtime_entrypoint() {{
  binary="$1"
  entrypoint="$2"
  test -L "$entrypoint"
  test "$(readlink "$entrypoint")" = "$RUNTIME_INSTALL_ROOT/current/$binary"
  test -x "$entrypoint"
}}

BOOTSTRAP_RELEASE="$RUNTIME_INSTALL_ROOT/releases/bootstrap-$seed_digest"
test ! -L "$BOOTSTRAP_RELEASE"
install -d -o root -g root -m 0755 "$BOOTSTRAP_RELEASE"
{stage}printf '%s\n' \
  '{{' \
  '  "schema": "cynic.runtime_seed.release_marker.v3",' \
  "  \"release_id\": \"bootstrap-$seed_digest\"," \
  '  "release_epoch": 0,' \
  '  "source_commit": null,' \
  '  "build_proof_digest": null,' \
  "  \"basket_digest\": \"sha256:$seed_digest\"" \
  '}}' > "$BOOTSTRAP_RELEASE/.cynic-runtime-release.json"
chmod 0644 "$BOOTSTRAP_RELEASE/.cynic-runtime-release.json"
{prepare}current_next="$RUNTIME_INSTALL_ROOT/.current.next.$$"
if test ! -e "$RUNTIME_INSTALL_ROOT/current" && test ! -L "$RUNTIME_INSTALL_ROOT/current"; then
  ln -s "$desired_release" "$current_next"
  mv "$current_next" "$RUNTIME_INSTALL_ROOT/current"
fi
{activate}{read_back}"#
    )
}

pub(super) fn verify_fragment(artifacts: &[BootstrapRuntimeArtifact]) -> String {
    if artifacts.is_empty() {
        return String::new();
    }
    let read_back = artifact_commands(artifacts, "read_back_runtime_entrypoint");
    format!(
        r#"read_back_runtime_entrypoint() {{
  binary="$1"
  entrypoint="$2"
  test -L "$entrypoint"
  test "$(readlink "$entrypoint")" = "$RUNTIME_INSTALL_ROOT/current/$binary"
  test -x "$entrypoint"
}}

{read_back}"#
    )
}
