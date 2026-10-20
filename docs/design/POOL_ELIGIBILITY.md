# Build Pool Eligibility — the two mandatory bricks

Schema: `cynic.pool.eligibility.v1`
Status: `ratified` — a node may not take a build job unless both bricks pass.

## 0. The incident that made this law

2026-10-19, Forgejo F-gate. A runner registered with the shared label
`cynic/build` picked up the gate job and died in 9 seconds:

```
Cannot find: node in PATH
❌  Failure - Post Checkout repository
```

The runner (`compute-01`) had git, curl, tar, python3 — no cargo, no rustc,
no node. The gate never reached the code: it failed on its own toolchain.
The same SHA was green on another runner minutes earlier, so the failure
looked like a flake and cost two hours of triage.

Two independent causes, therefore two independent bricks.

## 1. Brick one — declared pool labels

**Rule.** The labels a node may serve are declared in `infra/registry.json`
under the node's `pool.labels`. The runner's advertised labels are evidence,
not authority.

- A label advertised by the runner but not granted by the registry →
  **ineligible**. The runner is out of its lane.
- A label granted by the registry but not advertised by the runner →
  **ineligible**. The declaration is unbacked.

A runner may be narrowed at any time by re-registering it with fewer labels;
the registry stays the record of what the node is *allowed* to serve.

## 2. Brick two — proven toolchain

**Rule.** Before a node serves a label, it must have *proven* to provide the
tools that label's workflows need. The requirement set lives in the
registry under `pool.required_tools` (name, optional `min_version`,
`required` flag). Proof = the executable resolves in `PATH` and its version
banner is at least the declared minimum.

- Missing tool → **ineligible** for that label.
- Tool present but older than the minimum → **ineligible**.
- Version banner not comparable to the minimum → **ineligible** (unknown is
  not a pass).
- A tool declared but not required is probed and reported, never blocking.

Version banners are prose (`cargo 1.95.0 (f2d3ce0bd 2026-03-21)`), so the
first dotted numeric token is the version under comparison.

## 3. Where the rules live

- Pure domain: `crates/cynic-host/src/domain/pool_eligibility.rs`
  (`evaluate_pool_eligibility`, `compare_versions`, `extract_version`).
- Evidence adapter: `crates/cynic-host/src/adapters/local_toolchain_probe.rs`.
- CLI: `cynic build pool eligibility --node <id> --observed-labels a,b --json`.

The verb probes the host it runs on. To qualify a remote node, run the verb
on that node (or via an SSH transport) — the verdict always describes a host
that was actually observed.

## 4. Registry shape

```json
"pool": {
  "labels": ["cynic/build", "cynic/linux"],
  "required_tools": [
    {"name": "git",     "min_version": null,     "required": true},
    {"name": "cargo",   "min_version": "1.95.0", "required": true},
    {"name": "rustc",   "min_version": null,     "required": true},
    {"name": "node",    "min_version": "22.0.0", "required": true},
    {"name": "python3", "min_version": null,     "required": true}
  ]
}
```

Current declarations (2026-10-19):

| node | labels | required tools |
|---|---|---|
| `cynic-builder` (.119) | `cynic/build`, `cynic/linux` | git, cargo 1.95.0, rustc, node 22, python3 |
| `cynic-compute-01` | `cynic/compute-01` | git, python3 (no cargo/rustc/node yet) |
| `cynic-host-cortex` | `cynic/cortex-overflow` | git, cargo, rustc (RAM-gated) |

## 4.1 Capacity probe — proof before promotion

A node does not join a shared pool label because it *claims* the toolchain:
it proves it, with a workflow that runs on that node.

`.forgejo/workflows/probe-compute.yml` runs on the node's own label
(`cynic/compute-01`), reports the host capacity (cores, RAM, disk), prints the
toolchain versions, and compiles a real crate (`cargo test -p cynic-types`).
Only a green probe justifies promoting the node in the registry and
re-registering its runner with the shared labels.

Proof trail for `cynic-compute-01` (2026-10-19): probe run #27 green —
`cargo 1.95.0`, `cc (Debian 14.2.0-19)`, `node v22.23.3`, 12+8+0 tests passed,
416 G free on `/`. The registry promotion and the runner re-registration
(labels `cynic/build,cynic/linux,cynic/compute-01`) followed that green run.

The probe is a per-node workflow, not a pool one: it targets the specific
node label, so it can never be picked up by the shared pool.

## 5. Enforcement

- A scheduler must consult the eligibility verdict before dispatching. The
  verdict is the only place where "may this node take this job" is decided.
- Narrowing a runner's labels is the immediate mitigation when a node is not
  toolchain-ready: re-register with the narrow label set (as was done for
  `compute-01` on 2026-10-19) and record the reason in the registry.
- The network enrollment gate (`NETWORK_ENROLLMENT.md`) covers *access*; this
  document covers *capacity to do the work*. Both are mandatory.
