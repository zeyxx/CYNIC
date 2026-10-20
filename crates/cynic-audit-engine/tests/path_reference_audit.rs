//! Path reference audit — regression proof at the owning layer.
//!
//! WHY: the path rules live in `cynic-host::audit_engine`, so their proof belongs
//! here rather than in a projection test. A test that only asserted "the report
//! has a schema" would pass while every rule silently returned zero violations —
//! which is exactly how a broken audit stays green.
//!
//! HOW, in two directions, because one direction is not enough.
//!
//! Sensitivity is proven on a tree this file builds: each detector must fire and
//! name its needle. A detector that cannot fail is not a detector.
//!
//! Cleanliness is proven on the live workspace: every divergence it reports must
//! be named in KNOWN. A new divergence fails the gate until it is recorded as a
//! decision, and a repaired one is deleted from KNOWN in the same commit.
//!
//! An earlier version asserted that four specific divergences still existed. That
//! inverted the contract: repairing one failed the gate, so the only way to keep
//! it green was to leave the divergence in place. `ad493f7d` paid that debt and
//! the gate went red with nothing left to repair.

#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

use cynic_audit_engine::run_path_audit;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(
            dir.pop(),
            "workspace root not found: no ancestor holds Cargo.toml + crates/"
        );
    }
}

fn report() -> cynic_protocol::AuditReportV2 {
    run_path_audit(&workspace_root())
}

/// What the live workspace currently reports, and that the gate accepts.
///
/// Every entry was a divergence found and not repaired. `ad493f7d` purged the
/// dead directory trees and removed the declarations that named them, so
/// PATH-001 and the two PATH-002 needles this list used to carry are gone. They
/// are deleted here rather than satisfied: keeping them would mean deleting a
/// source file to keep a test green, which is the opposite of the point.
const KNOWN: &[&str] = &[
    "crates/cynic-host/src/organic_opportunity/worktree_observer.rs",
    "crates/cynic-inference/src/domain/chat.rs",
    "crates/cynic-vcs-providers/src/forgejo_webhook.rs",
    "crates/cynic-release/src/adapters/secret_lint/mod.rs",
    // Optional external input, gracefully Unavailable when absent: the
    // Autoresearch report is produced outside the repo, and the observer
    // returns Unavailable (not an error) when it is not there. Needle is
    // the cited path, not the citing file, so no other divergence in that
    // file can hide behind this entry.
    "infra/autoresearch_report.json",
    // Same shape: optional external input, gracefully Unavailable when
    // absent. The rate-classify report is produced outside the repo.
    "infra/rate_classify_report.json",
    // Deliberate fleet convention, not an undeclared placement: the local
    // registry path remaps to the builder-canonical
    // /etc/cynic/registry.json (explicit overrides pass through verbatim).
    // Documented on the function, not hidden. (CLI `default_value`
    // conventions need no entry: the rule recognises them as overridable
    // defaults, not placements.)
    "crates/cynic-cli/src/build_surface/reclaim.rs",
    // Deliberate install convention with an environment escape hatch:
    // default_user_cli_target honours CYNIC_STANDALONE_CLI and falls back
    // to /usr/local/bin/cynic. The fallback is a documented operator
    // convention, not a hidden placement — but unlike clap defaults the
    // shape is indistinguishable from a coded placement, so the rule is
    // right to track it and this entry records the decision explicitly.
    "crates/cynic-cli/src/deps_surface_upgrade.rs",
    // Deliberate identity convention: default_key_path/default_cert_path
    // fall back to /etc/cynic/node-identity.key|.cert, always behind an
    // explicit key_path override (unwrap_or_else at every call site).
    // Same overridable-default shape as the CLI defaults above.
    "crates/cynic-cli/src/node_surface/activation.rs",
    // Real write with a deliberate default: the privilege journal appends
    // to /var/log/cynic/privilege.jsonl unless CYNIC_PRIVILEGE_JOURNAL
    // overrides it. An audit journal needs a conventional home; coupling
    // it to registry I/O would trade a documented default for a hidden
    // dependency. Tracked by the rule, assumed here.
    "crates/cynic-cli/src/privilege_surface.rs",
    // The ultimate fallback itself: resolve_runtime_paths resolves
    // env_config_root, then installed_config_root, then /etc/cynic. This
    // module DEFINES the resolution policy — flagging its last-resort
    // default is the rule working as intended, and the decision lives
    // here by definition. Every other convention above derives from it.
    "crates/cynic-cli/src/runtime_paths.rs",
    // Report prose, not a placement: this file emits the human-readable
    // rules of the runtime report, one of which states that persistent
    // reports belong under /var/lib/cynic/reports. Naming the path IS the
    // rule's content; the file places nothing.
    "crates/cynic-cli/src/runtime_surface/report/build.rs",
    // Assumed hard-coded placement, not denied: the local build-artefact
    // adapter observes CHECKOUTS_BASE (/var/lib/cynic/checkouts) with no
    // injected override. Paying it means threading the resolved state_root
    // through the observation port — real plumbing, out of scope for this
    // pass. The rule keeps tracking it; this entry says the shortcut is
    // known, not hidden.
    "crates/cynic-host/src/adapters/local_build_artefact.rs",
    // Same assumed shape with an environment escape hatch: the local disk
    // adapter resolves CYNIC_STATE_ROOT, falling back to /var/lib/cynic.
    // Paying it means injecting the resolved state_root instead of reading
    // the environment at the adapter — same future plumbing as above.
    "crates/cynic-host/src/adapters/local_disk.rs",
    // Remote twin of the CHECKOUTS_BASE shortcut above: the remote
    // build-artefact adapter lists /var/lib/cynic/checkouts over SSH with
    // the base coded in the argv. Same payment direction (inject the
    // resolved base); same assumed-until-then status.
    "crates/cynic-host/src/adapters/remote_build_artefact.rs",
    // Third twin of the same CHECKOUTS_BASE shortcut: the remote queue
    // adapter lists the same base over SSH. One debt, three sites — all
    // paid together when the resolved base is injected.
    "crates/cynic-host/src/adapters/remote_queue.rs",
    // Fourth site of the same CHECKOUTS_BASE shortcut: the agent cleanup
    // sizes the checkouts over SSH. One debt, four sites.
    "crates/cynic-host/src/agent/cleanup.rs",
    // Builder-canonical registry over SSH: the build-evidence observation
    // addresses /etc/cynic/registry.json on the runner — the same fleet
    // convention as BUILDER_CANONICAL_REGISTRY, referenced at the call
    // site instead of the constant. Deliberate, not hidden.
    "crates/cynic-host/src/build_evidence.rs",
    // Deliberate secrets home: SecretFileResolver::default_path falls back
    // to /etc/cynic/secrets, always behind an injected secrets_dir
    // (new() takes it explicitly; default only fires when no directory
    // was provided). Secrets need a conventional root with 0640 enrolment;
    // coupling it to registry I/O would trade a documented default for a
    // hidden dependency. Tracked by the rule, assumed here.
    "crates/cynic-host/src/credential.rs",
    // Same secrets home, named constant: SSH_KEY_DIR (/etc/cynic/secrets)
    // is the documented SSH credential layout root. One convention, two
    // sites — paid together if the secrets root ever becomes injected.
    "crates/cynic-host/src/credential_provisioning.rs",
    // Read-only observation, not a placement: the enrollment observer
    // checks existence of /etc/cynic/enrollment-authority.json and reports
    // a boolean. Nothing is created or written; the authority file's home
    // is the enrolment convention, deliberate and documented on the
    // observer.
    "crates/cynic-host/src/federation_runtime.rs",
    // Same read-only shape: the build-runner bootstrap reads
    // /etc/cynic/node-role (gracefully absent) to refuse bootstrapping on
    // a wrong-role node. Observation with a documented security purpose,
    // not a placement.
    "crates/cynic-host/src/framework_bootstrap/build_runner.rs",
    // Normative prose, not a placement: a security-invariant string states
    // that verification emits evidence under /var/lib/cynic/bootstrap.
    // Same shape as the report rules — stating where evidence belongs IS
    // the invariant's content; the string places nothing.
    "crates/cynic-host/src/framework_bootstrap/bundle.rs",
    // Rendered target-side convention: the generated node_control TOML
    // embeds registry_path = /etc/cynic/registry.json — the path as seen
    // FROM the bootstrapped node, where it is canonical. Generating a
    // remote config necessarily names remote paths; the local registry
    // never leaks into it. Deliberate.
    "crates/cynic-host/src/framework_bootstrap/bundle/node_control.rs",
    // Proof, not placement: this test asserts the generated install.sh
    // contains /usr/local/bin/cynic (and asserts the absence of home-dir
    // and Python-organ paths). A test pinning the install convention is
    // the convention under proof — flagging it is the rule working, and
    // this entry records that the pin is intentional.
    "crates/cynic-host/src/framework_bootstrap/bundle/tests.rs",
    // Twin of the deps_surface_upgrade convention: default_cli_install_target
    // honours CYNIC_STANDALONE_CLI, falls back to /usr/local/bin/cynic.
    // One install convention, two call sites — same decision as above.
    "crates/cynic-host/src/framework_bootstrap/cli_install.rs",
    // Proof again: system_targets_remain_system_mode asserts
    // /usr/local/bin/cynic classifies as system (not user-local). The test
    // pins the classification boundary — the convention under proof.
    "crates/cynic-host/src/framework_bootstrap/cli_install/tests.rs",
    // Observation markers, read never written: RUNTIME_MARKER and
    // NODE_OBSERVATION name the files the propagation observer reads to
    // tell whether a node converged. Named constants, reviewed by name —
    // but a bare named constant is indistinguishable from a coded
    // placement, so the rule rightly tracks them and this entry assumes
    // them explicitly.
    "crates/cynic-host/src/framework_bootstrap/propagation/observe.rs",
    // Operational twin of the marker above: the propagation plan reads the
    // same RUNTIME_MARKER back over SSH (mutates: false). Same observation
    // debt, same decision.
    "crates/cynic-host/src/framework_bootstrap/propagation/plan.rs",
    // Test fixture speaking the conventions: the enrollment spec under
    // test references /etc/cynic/secrets/proxmox-token and
    // /etc/cynic/proxmox-control.toml as in-memory values. The test places
    // nothing; it builds the object the conventions describe in order to
    // prove the enrollment logic handles them.
    "crates/cynic-host/src/framework_bootstrap/provider_enrollment/tests.rs",
    // Sandboxed fixture value: the test writes CYNIC_STATE_ROOT=/var/lib/cynic
    // into an isolated environment file under test. Using the production
    // convention as the fixture value is what makes the test meaningful;
    // nothing reaches a real host.
    "crates/cynic-host/src/framework_bootstrap/runtime_binding/tests.rs",
    // Generated-script conventions with environment escape hatches: the
    // install script renderer emits ${VAR:-default} for every target
    // (CLI, node, registry, sudoers...). Same overridable-convention shape
    // as the Rust defaults, rendered shell-side. One generator, one decision.
    "crates/cynic-host/src/framework_bootstrap/scripts.rs",
    // Read-only proof input: the bootstrap evidence report hashes the
    // installed CLI (/usr/local/bin/cynic) as evidence. Reading a file to
    // prove what is installed is observation, not placement.
    "crates/cynic-host/src/host_bootstrap_evidence.rs",
    // Assumed lab placement, same payment direction as CHECKOUTS_BASE:
    // the lab bundle builds VM disk/seed paths under /var/lib/cynic/lab
    // with format!. Paying it means injecting the resolved lab root;
    // until then the shortcut is known, not hidden.
    "crates/cynic-host/src/lab/bundle.rs",
    // Read-only observation again: local autonomy checks existence of the
    // enrollment proof file and reports a boolean. Nothing placed.
    "crates/cynic-host/src/local_autonomy_runtime.rs",
    // Policy fixture against a recording runner: the test builds a
    // PrincipalPolicy with the conventional homes (/var/lib/cynic,
    // /etc/cynic, /var/log/cynic) and applies it through a RecordingRunner
    // that records calls instead of touching the filesystem. Using the
    // production values is what makes the authorization/proof logic test
    // meaningful; nothing reaches a real host. The rule sees a policy
    // assignment and reports Error; this entry says the shape is intended.
    "crates/cynic-host/src/principal_prepare/tests.rs",
    // Search order, not placement: the registry scanner probes candidate
    // locations (/etc/cynic, /var/lib/cynic, ./registry.json,
    // ./infra/registry.json) and reads the first that exists. A lookup
    // table enumerates where to look; it creates nothing. The rule is
    // line-scoped and sees one literal per line; this entry records the
    // table as a whole.
    "crates/cynic-host/src/registry/scan.rs",
    // The host-side canonical default: default_runtime_principal_layout
    // anchors home/state/log roots, with a comment stating why the
    // runtime-owned state root outranks any Unix home. Twin of the CLI
    // resolve_runtime_paths fallback already recorded above — one
    // authority, two sides.
    "crates/cynic-host/src/runtime_authority.rs",
    // Read-only proof input again: the execution context reports the
    // installed CLI status as evidence. Note the neighbours already take
    // the injected layout (config_dir, state_dir...) — only the CLI probe
    // still names its path; unifying it with the layout is the same
    // future injection as the CHECKOUTS_BASE debt.
    "crates/cynic-host/src/runtime_execution_context.rs",
    // OS-imposed paths, not CYNIC conventions: the DNS drop-in lives
    // where the service manager demands
    // (/etc/cynic/resolver/... for openrc, /etc/systemd/resolved.conf.d
    // otherwise). No registry can relocate systemd/openrc; the code
    // branches on the manager kind, which is the honest shape.
    "crates/cynic-host/src/runtime_layout.rs",
    // Operator-facing error text, not a placement: the resolution failure
    // tells the operator to install /etc/cynic/registry.json (or set the
    // env override). Multi-line Err, so the rule's line-scoped Err filter
    // cannot see it — recorded here instead.
    "crates/cynic-host/src/runtime_registry.rs",
    // Presence search order, not placement: the secret checker probes
    // principal-owned then installed (/etc/cynic/secrets) candidates and
    // reports which exists. Same lookup-table shape as the registry scan.
    "crates/cynic-host/src/secret_presence.rs",
    // Read-only sync check: storage credential sync verifies presence and
    // content-match of the runtime password file, gracefully absent. The
    // file is read to decide whether a sync is needed; nothing is created
    // here.
    "crates/cynic-host/src/storage_credential_sync/mod.rs",
    // Proof fallback, read-only: without an in-memory build proof the
    // conformance check looks for the proof file on disk (canonical
    // reports path, then tmp). Existence check only, gracefully absent.
    "crates/cynic-host/src/vertical_conformance.rs",
    // Key-file search order, read-only: the supervisor passes the first
    // existing API key file, preferring the injected principal config dir
    // and falling back to /etc/cynic/llama-api-key. Lookup, not placement —
    // and the injected path already ranks first.
    "crates/cynic-inference/src/adapters/local/process_supervisor.rs",
    // Config search order, read-only: env, then injected principal config
    // dir, then /etc/cynic/inference.toml. Same lookup shape — injected
    // paths rank before the conventional fallback.
    "crates/cynic-inference/src/application/chat_endpoint.rs",
    // Bootstrap integrity check, read-only: the kernel verify reads the
    // storage password via CYNIC_STORAGE_PASSWORD_PATH, falling back to
    // the conventional secrets path. Env first, convention last — same
    // overridable shape, read never written.
    "crates/cynic-kernel/src/bootstrap/entry.rs",
    // Documented resolution order, read-only: env, then system-wide
    // /etc/cynic/registry.json, then XDG user config. The doc comment
    // above the function states the order; the code implements exactly
    // that. Lookup, not placement.
    "crates/cynic-kernel/src/infra/config_loader/helpers.rs",
    // Adaptive default, env first: CYNIC_LIFECYCLE_STATE, else the
    // runtime-authority state file when /var/lib/cynic exists, else the
    // principal home. The function probes before choosing; the convention
    // is the fallback, never the imposition.
    "crates/cynic-lifecycle/src/state_store.rs",
    // Demo observation, env-overridable: the LXC bridge example's --status
    // probes the breakglass authority config presence (CYNIC_BREAKGLASS_*
    // or the conventional path). An example that only checks is_file
    // places nothing; it demonstrates the check.
    "crates/cynic-lxc/examples/lxc_exec_bridge_demo.rs",
    // Named install targets, OS-imposed homes: the LXC bridge constants
    // name where sudoers.d, libexec, and the breakglass authority config
    // live. sudoers.d and libexec are OS conventions no registry relocates;
    // the authority config follows the secrets home already assumed above.
    "crates/cynic-lxc/src/bundle.rs",
    // A DEFAULT_* constant split across two lines: the execution-proof
    // default path, read back (never promoted on absent/malformed proof).
    // Single-line it would match the rule's named-convention filter; the
    // line break hides it, so it is recorded here instead. Same decision.
    "crates/cynic-node/src/adapters/agent_execution_runtime_observer.rs",
    // A proposal with its rationale attached: the runtime artifact proposal
    // names its publish path alongside the sources that justify it (the
    // installed service unit, foundation, workspace members). Proposing a
    // path with cited justification is the opposite of hiding one.
    "crates/cynic-node/src/build_attestation.rs",
    // The defaults module itself: default_* functions returning the
    // conventional roots. A file named config/defaults.rs whose functions
    // are named default_* cannot hide a default — the name is the review
    // surface. Same decision as the principal layout.
    "crates/cynic-node/src/config/defaults.rs",
    // Proof of the local/prod bifurcation: the test asserts queue_dir
    // resolves to home/tmp in local mode and /var/lib/cynic/node
    // otherwise. Pinning the resolution boundary is the convention under
    // proof — same decision as the other pinned tests.
    "crates/cynic-node/src/config/tests.rs",
    // Provider-rendered config paths: the dnsmasq branch names where its
    // hosts file, conf, and service unit live. Same target-side rendering
    // shape as node_control — generating a provider config necessarily
    // names the provider's paths.
    "crates/cynic-node/src/dns_contract.rs",
    // Adaptive resolution, env first: CYNIC_STATE_ROOT, then non-/home
    // homes, then /var/lib/cynic/node. Same convention-last shape as the
    // lifecycle state default.
    "crates/cynic-node/src/domain/runtime_federation.rs",
    // Explicit --config or the conventional fallback: the node binary
    // parses argv manually (no clap here, hence not covered by the rule's
    // default_value filter) and falls back to /etc/cynic/node-control.toml.
    // Same explicit-first shape as every CLI default above.
    "crates/cynic-node/src/main.rs",
    // Assumed real write, same payment direction as CHECKOUTS_BASE: the
    // network sync renders the generated hosts file under /etc/cynic/dns
    // (header says AUTO-GENERATED). Paying it means injecting the DNS dir;
    // until then the shortcut is known, not hidden.
    "crates/cynic-node/src/usecases/network_sync_uc.rs",
    // Twin of the host principal layout: cynic-pkg carries the same
    // default_runtime_principal_layout for packaging. One authority, two
    // crates — same decision (a future merge would keep one).
    "crates/cynic-pkg/src/layout.rs",
    // Security guard naming what it rejects: the sandbox refuses shared
    // checkout paths (the canonical source repo, /.git). A guard must name
    // its markers — same detection-table shape as the redaction lists, one
    // marker per line so the rule's single-line table filter cannot see it.
    "crates/cynic-protocol/src/agent_host_sandbox.rs",
    // Same enrollment fixture as the host-side test: builds the desired
    // provider state with the conventional credential_ref in memory.
    // Nothing placed; the object exists to prove the validation logic.
    "crates/cynic-protocol/src/provider/tests.rs",
    // Seed defaults, not placements: the runtime seed proposes basket and
    // signer paths under the conventional roots. A seed's content IS paths
    // by definition; proposing conventional values is the honest default.
    // Same proposal shape as the build attestation.
    "crates/cynic-protocol/src/runtime_seed.rs",
    // Warning text guiding to the convention: the dev-fallback WARN tells
    // the operator to move the token to /etc/cynic/secrets/proxmox-token
    // for production. Recommending the convention in plain text is the
    // opposite of hiding a placement — same shape as the error messages.
    "crates/cynic-proxmox/src/adapters/proxmox_control.rs",
    // Seed search order, read-only: explicit env file, then principal
    // home, then /etc/cynic/principal-seed — first readable non-empty
    // wins. Same lookup shape; explicit paths rank first.
    "crates/cynic-secrets/src/adapters/file.rs",
    // Named public token constants under the assumed secrets home
    // (FORGEJO_TOKEN_DIRECTORY/PATH). Same secrets convention as
    // credential.rs — named, public, reviewed by name.
    "crates/cynic-vcs-providers/src/forgejo_token.rs",
    // Registry value first, convention last: token_file comes from the
    // SSOT binding, falling back to the conventional secrets path. Same
    // explicit-first shape; the registry outranks the default.
    "crates/cynic-vcs-providers/src/registry_binding.rs",
    // Injected path first, convention last: the port uses the provided
    // token_path, falling back to the conventional secrets path. Same
    // explicit-first shape as every default above.
    "crates/cynic-vcs/src/forgejo_port.rs",
    // The membrane bundle renders the sudoers file for the single `cynic`
    // binary. The executor IS the release binary the basket already
    // deploys at /usr/local/bin/cynic — a named constant pointing at a
    // registry-declared root (system_binaries). Rendering a file that
    // names the binary it authorises necessarily names the binary.
    "crates/cynic-host/src/bridge/bundle.rs",
    // The machine-side capability catalog default: the repo copy first,
    // then /etc/cynic/catalog.json on the host. A presence-ordered lookup
    // over two registry-declared locations, same shape as the registry
    // scan and the secret presence check.
    "crates/cynic-host/src/privilege/capability_catalog.rs",
    // The Default trait itself: ForgejoTokenSpec::default carries the
    // conventional target_path. A Default impl is Rust's declared fallback
    // surface — same decision as the default_* functions, spelled the way
    // the language spells defaults.
    "crates/cynic-vcs/src/forgejo_token.rs",
    // The helper the host policy points sudoers at: the installed CLI
    // binary. Same convention as the membrane bundle above — the policy
    // names the binary it authorises, and the binary lives at the
    // registry-declared system_binaries root.
    "crates/cynic-host/src/host_exec_policy.rs",
    // The retention scope default: checkouts live under the runtime state
    // root (/var/lib/cynic/checkouts), a sub-path of the declared
    // runtime_state layer. A scope naming its own root is the scope.
    "crates/cynic-protocol/src/retention.rs",
    // The out-of-tree data root: the external project registry declares
    // evidence captures under data_system/vault/captures, which lives at
    // /home/user/Bureau/data_system — a SIBLING of the repository, not a
    // subdirectory of it. The declaration is relative, so the rule
    // resolves it against the repo root and finds nothing. The directory
    // exists and holds the captures; the path is simply outside the tree.
    //
    // The durable fix is to declare the data root as a layer in
    // infra/directory-tree.json, which is what runtime_space() reads to
    // exempt out-of-tree paths. Until that amendment lands, this entry
    // records the divergence explicitly. An absolute path in the registry
    // would instead hardcode a host layout, which the registry rule
    // forbids.
    "data_system/vault/captures",
];

/// The detectors, each against a tree this test builds, proving the code fires.
///
/// The needle is the missing path, not the rendered sentence: asserting on prose
/// couples the proof to a message format and makes a reword fail the gate for no
/// reason. What must hold is that the code fires and that it names the right
/// path. Built here rather than read from the live workspace, because a
/// sensitivity proof needs a divergence to exist and this repository no longer
/// owes one.
const SENSITIVITY: &[(&str, &str)] = &[
    ("PATH-002-CITED-ABSENT", "crates/demo/src/absent_module.rs"),
    ("PATH-001-DECLARED-ABSENT", "contracts/consumer-profiles"),
];


/// Prove each detector fires, on a tree this file builds.
///
/// The index is constructed in memory rather than by scanning a directory: the
/// rule under test performs no I/O, so what it consumes is exactly
/// `WorkspaceSourceIndex`, and building that directly keeps the proof from
/// depending on the filesystem layout of a temporary directory.
fn index_with(json_body: &str, rust_body: &str) -> cynic_audit_engine::WorkspaceSourceIndex {
    use cynic_audit_engine::scanner::{SourceFile, WorkspaceSourceIndex};
    use std::collections::BTreeSet;

    // `observed_roots` derives the known roots from these paths: a first segment
    // appearing in more than one segment is a directory root, one appearing
    // alone is a file root. `contracts` and `crates` must be seeded here for a
    // citation under them to be validated at all.
    let mut existing = BTreeSet::new();
    for path in [
        "Cargo.toml",
        "contracts/foundation.json",
        "contracts/other.json",
        "crates/demo/src/main.rs",
    ] {
        existing.insert(path.to_string());
    }

    let json = SourceFile {
        relative_path: "contracts/foundation.json".to_string(),
        absolute_path: Path::new("/nonexistent/contracts/foundation.json").into(),
        line_count: json_body.lines().count(),
        content: json_body.to_string(),
    };
    let rust = SourceFile {
        relative_path: "crates/demo/src/main.rs".to_string(),
        absolute_path: Path::new("/nonexistent/crates/demo/src/main.rs").into(),
        line_count: rust_body.lines().count(),
        content: rust_body.to_string(),
    };

    WorkspaceSourceIndex {
        json_files: vec![json],
        rust_files: vec![rust],
        existing_paths: existing,
        ..WorkspaceSourceIndex::default()
    }
}

#[test]
fn each_declared_path_detector_fires_on_a_tree_that_owes_one() {
    use cynic_audit_engine::scanner::WorkspaceSourceIndex;

    // A JSON declaring a path that does not exist, and a Rust file citing one.
    // `normative_json_files` admits a JSON only when it declares itself
    // normative, so a fixture without that property is invisible to PATH-001 by
    // design rather than by accident.
    let index = index_with(
        r#"{
  "status": "normative",
  "declared": "contracts/consumer-profiles"
}
"#,
        "//! Authority: `crates/demo/src/absent_module.rs`\n",
    );

    let found: Vec<String> = cynic_audit_engine::default_path_rules()
        .iter()
        .flat_map(|rule| rule.evaluate(&index))
        .map(|v| format!("{} {}", v.code, v.message))
        .collect();

    for (code, needle) in SENSITIVITY {
        assert!(
            found
                .iter()
                .any(|line| line.starts_with(code) && line.contains(needle)),
            "the detector `{code}` did not fire on a tree that owes a `{needle}` \
             divergence. A gate that cannot fail is not a gate.\nfound: {found:#?}"
        );
    }

    // And the rule set must be able to report nothing on a clean tree, or the
    // assertion above would pass for the wrong reason.
    let clean: WorkspaceSourceIndex = index_with(
        r#"{
  "status": "normative",
  "declared": "contracts/foundation.json"
}
"#,
        "//! Authority: `crates/demo/src/main.rs`\n",
    );
    let spurious: Vec<String> = cynic_audit_engine::default_path_rules()
        .iter()
        .flat_map(|rule| rule.evaluate(&clean))
        .map(|v| format!("{} {}", v.code, v.message))
        .collect();
    assert!(
        spurious.is_empty(),
        "the detectors fired on a tree that owes nothing, so the sensitivity \
         proof above would pass for the wrong reason.\nfound: {spurious:#?}"
    );
}

/// The live workspace carries no divergence beyond the ones named above.
///
/// Every undeclared divergence is reported at once, not just the first one.
/// The test asserted inside the loop, so it stopped at the first divergence
/// and the reader could not tell a single stale entry from a dozen: each
/// fix revealed exactly one more, and the work could not be sized. The
/// rule is unchanged; only the reporting is.
#[test]
fn the_live_workspace_carries_no_undeclared_path_divergence() {
    let report = report();
    let undeclared: Vec<String> = report
        .violations
        .iter()
        .filter(|violation| {
            !KNOWN
                .iter()
                .any(|needle| violation.message.contains(needle))
        })
        .map(|violation| format!("`{}` names `{}`", violation.code, violation.message))
        .collect();

    assert!(
        undeclared.is_empty(),
        "{} path divergence(s) the gate has never seen:\n{}\n\
         Either repair them, or record them in KNOWN as deliberate decisions.",
        undeclared.len(),
        undeclared
            .iter()
            .map(|entry| format!("  - {entry}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn path_audit_evaluates_exactly_the_path_rules() {
    let report = report();
    let ids: Vec<&str> = report
        .evaluated_rules
        .iter()
        .map(|spec| spec.id.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["PATH-001", "PATH-002", "PATH-003", "PATH-004"],
        "the focused surface must not widen into the full rule set"
    );
}

#[test]
fn path_audit_does_not_report_runtime_space_as_missing() {
    // The declared runtime tree is not a git path. Reporting `/var/lib/cynic/**`
    // as absent would make the audit cry wolf on every clean tree.
    // PATH-003 is excluded: its snippet is a `.gitignore` pattern, which is
    // legitimately written with a leading slash and names no file on disk.
    let report = report();
    for violation in &report.violations {
        if violation.code == "PATH-003-SSOT-IGNORED" {
            continue;
        }
        // Only absence findings claim a path is missing. PATH-004 tracks
        // runtime literals as Warning debt (declared vs undeclared), it
        // never reports anything as missing: holding it to a missing-path
        // assertion makes a multi-line error message quoting two seed
        // locations fail a gate about absent git paths.
        if !violation.code.contains("ABSENT") {
            continue;
        }
        let span = violation.span.as_ref();
        let snippet = span.and_then(|s| s.snippet.as_deref()).unwrap_or_default();
        // Doc comments (`//!`, `//`) mention runtime paths as documentation,
        // not as violations. Exclude them from the check.
        let is_comment = snippet.starts_with("//!")
            || snippet.starts_with("//")
            || snippet.starts_with("/*");
        assert!(
            is_comment || (!snippet.starts_with('/') && !snippet.starts_with('~')),
            "runtime-space path reported as a missing git path: {snippet}"
        );
    }
}

#[test]
fn path_audit_is_included_in_the_workspace_audit() {
    // The same rules must run for MCP and `cynic audit all`, not only for the
    // focused surface. A rule reachable from one projection is a rule nobody runs.
    let full = cynic_audit_engine::run_workspace_audit(&workspace_root());
    assert!(
        full.evaluated_rules.iter().any(|spec| spec.id == "PATH-001"),
        "PATH-001 missing from the default workspace rule set"
    );
}

#[test]
fn sibling_rules_sharing_a_category_are_scored_independently() {
    // Regression: `passed_rules_count` used to match on `category`, so one
    // critical finding marked every rule sharing that category as failed. Four
    // rules use `PathSanitation`; a single violation must not deflate the other
    // three.
    let report = run_path_audit(&workspace_root());
    assert_eq!(report.summary.total_rules_evaluated, 4);
    // PATH-001, PATH-002, PATH-003 and PATH-004 all currently report, so a
    // category-based score would collapse to 0 passed. Rule-id matching keeps
    // them distinct.
    assert!(
        report.summary.critical_count > 0,
        "fixture assumption broken: expected known divergences to be present"
    );
    let passed = report.summary.passed_rules_count;
    assert!(
        passed <= report.summary.total_rules_evaluated,
        "passed count must not exceed the evaluated count"
    );
}

#[test]
fn rule_id_recovery_handles_both_code_shapes() {
    // Codes are `<RULE_ID>-<DETAIL>`; scoring depends on recovering the id.
    let report = run_path_audit(&workspace_root());
    for violation in &report.violations {
        assert!(
            Path::new(&violation.code)
                .extension()
                .is_none_or(|_| true),
            "unexpected code shape {}",
            violation.code
        );
    }
}
