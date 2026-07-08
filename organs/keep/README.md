# CYNIC organ-keep

`organ-keep` owns storage durability: primary storage activation, backup,
migration and restore contracts.

Current focus: activate `cynic-lxc-surrealdb` as dedicated storage primary
without legacy shell provisioning.

Contract:

```text
axioms/storage-primary-activation-v1.json
```

Build the non-mutating activation bundle:

```bash
make storage-primary-bundle out=/tmp/cynic-storage-primary
```

The bundle is not an installer. It produces deterministic files and a manifest
that become installable only after:

- `fleet.node.status` observes `lxc/103` running;
- LXC exec bridge status is `active`;
- Synapse/IPAM for `organ-surrealdb.synapse.cynic` is valid.

Check activation status:

```bash
make storage-primary-status
```

Optional evidence inputs:

```bash
make storage-primary-status args="--bundle-dir /tmp/cynic-storage-primary --bridge-status-json /tmp/bridge.json --fleet-report-json /tmp/fleet.json --health-json /tmp/health.json"
```
