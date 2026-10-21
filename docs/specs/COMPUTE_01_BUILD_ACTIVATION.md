# Compute-01 Build Activation

Closes #228. Turns `cynic-compute-01-build` from `planned` into an
`observed` pool member. Each step produces evidence; no step is skipped.

Status model (PROSE-001: code and probes are authority, never docs):
base identity first, then pool eligibility, then role — each gated on
its own probe. `observed` requires probe evidence (rule `4e088bc3`);
the pool health check is the executable gate
(`crates/cynic-build/src/builder_pool.rs`).

## 0. Where this runs

On compute-01 itself (Debian 13 trixie, 192.168.0.25, AMD Ryzen 7 5700U,
16 GB). As user `titouan`, sudo only for the systemd unit. Nothing here
runs from cortex except the probes in step 5.

## 1. Toolchain (pool eligibility, brick 1)

The pool requires a proven toolchain plus declared labels
(`cynic/build,cynic/linux`). Reproduce the builder layout
(`/opt` toolchains, `/usr/local/bin` symlinks, see registry entry
`cynic-builder`):

```sh
cargo --version
rustc --version
```

Record both version strings. They are the toolchain evidence for step 6.
If rustup is absent, install the same channel the builder uses, then
re-run the two commands.

## 2. Receiver binary (pool eligibility, brick 2)

Port `:9090` on the builder is served by the `builder_http` surface
(`crates/cynic-node/src/usecases/builder_http.rs`: `/health`,
`/workloads/submit`, `/proofs/{id}`, `/records/{id}`).

The reference systemd unit (`infra/systemd/cynic-node.service`) is
declared but absent from the tree (debt ledger PATH-001). Do not invent
one: read the live unit off the working builder first:

```sh
systemctl list-units | grep -i cynic
systemctl cat <unit>            # ExecStart, StateDirectory, User
ss -tlnp | grep 9090           # which binary holds the port
```

Reproduce the same unit on compute-01 (same binary, same state root
layout `/var/lib/cynic/...`), enable it, start it:

```sh
sudo systemctl enable --now <unit>
sudo systemctl is-active <unit>
```

## 3. Enrollment (base layer)

Base enrollment answers three facts: which machine (stable id in
`infra/registry.json`), how to reach it (declared transports), who
vouches for it (dated stamp). Before touching the registry, prove
the kernel handshake from compute-01:

- Request a PreAuthKey for the machine, complete the handshake.
- Known trap (`f829fb79`): the kernel WS route requires role `organ`
  or `internal`. A valid signature with any other role returns 403.
  If you get 403, fix the requested role, do not retry the key.

Record: date, enroller, review date (builder precedent:
`enrolled_by: forgejo-runner builder-119`, review +30 days).

## 4. Registry deposit

Edit `infra/registry.json`, node `cynic-compute-01-build`:

- add `enrollment: {date, enrolled_by, review_after}`,
- add observed `toolchain: {cargo, rustc}` under the node,
- flip `deploy_target` and `boot_persistence` to true once step 2
  survives a reboot,
- keep `status: planned` until step 5 passes.

## 5. Probe (the gate)

From any machine that routes to the LAN (cortex works, verified
2026-10-21):

```sh
curl -s -m 5 -o /dev/null -w '%{http_code}\n' http://192.168.0.25:9090/health
cynic discovery nodes        # registry read-back: the node must be listed
cynic network shadow         # declared-vs-observed drift: must not flag the node
```

Expected: `200`, node listed, no drift on the node. Paste all three
outputs into #228. Only then flip `status` to `observed` in the
registry: status follows observation, never precedes it.

Note: `cynic build pool eligibility` and `cynic enroll status` do not
exist in the tree; the three commands above are their honest
equivalents today (see Known gaps).

## 6. Pool confirmation

`resolve_builder_pool_with_health` (`crates/cynic-build/src/builder_pool.rs`)
lists the node as healthy. One submitted build routed to compute-01
with its canonical gate report closes the loop and the issue.

## Known gaps (structural, tracked in `infra/audit-debt-ledger.json`)

- No `cynic build pool eligibility` verb: the health check lives in
  `builder_pool.rs` only, with no CLI surface. Until one exists, the
  curl probe above is the eligibility proof.
- No `cynic enroll status` verb and no `cynic-node-enroll` binary: the
  enrollment request/grant files exist on the builder but no in-tree
  tool produces them. Until one exists, enrollment is a manual
  kernel handshake, and `cynic discovery nodes` plus `cynic network
  shadow` are the status read-backs.
- No reference systemd unit in the tree (debt PATH-001), and the
  working builder itself runs outside systemd. Persist units on both
  machines after the manual launch proves the invocation.
