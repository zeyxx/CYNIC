# CYNIC Infra

`infra/` contains deployment and runtime placement assets for the low-layer
organism baseline.

Keep:

- `registry.json`: active node and lifecycle placement.
- `registries/lab.json`: portable non-canonical lab topology for N-case smoke
  tests and VM/LXC experiments.
- `runtime-authority.json`: runtime surface authority map.
- `agent-exposure.json`: active Rust MCP exposure.
- `systemd/`: active service templates.
- `surrealdb/` and `surreal_init.surql`: storage schema/bootstrap.
- `compose/`: local SurrealDB helper.
- `firewall/`: host firewall projection.
- `sudoers.d/`: bounded OS operator projection.
- `repo-access-profile.json`: repository access profile.

Do not put runtime snapshots, organ state, UX containers, provider experiments,
or narrative backlogs here. A new infra file must either be referenced by the
registry/systemd, configure storage/security, or support an executable runtime
check.

Use `make lab-smoke` before promoting runtime assumptions into
`infra/registry.json`. The lab registry is deliberately not the fleet source of
truth; it exists to prove that crates and CLI surfaces do not hardcode the real
fleet.
