# Base Enrollment — the precondition for any role

Schema: `cynic.base_enrollment.v1`
Status: `ratified` — no node may claim a role, be attributed a pool label, or
receive a workload before it is base-enrolled.

## 0. What "base enrollment" means

Base enrollment is the *minimum* identity a machine has before it is
allowed to be anything. It answers three questions, all of them factual:

1. **Which machine is it?** — a stable node id in `infra/registry.json`.
2. **Can we reach it, and how?** — declared planes and transports.
3. **Who vouches for it, and until when?** — an enrollment stamp with a
   date, an enroller, and a review date.

Everything else a node can be (a builder, an embedding server, an agent
host) is an *attribution* layered on top of that base. Attribution without
base enrollment is a claim, not a fact.

## 1. The three layers, in order

| Layer | Question | Where it lives | Blocks |
|---|---|---|---|
| **Base enrollment** | Is this machine known, reachable, accountable? | `nodes[].id`, `nodes[].network`, `nodes[].transports`, `nodes[].enrollment` | any role, any label, any workload |
| **Pool eligibility** | May this node serve label X with this toolchain? | `nodes[].pool` + `cynic build pool eligibility` | taking a build job |
| **Role attribution** | What is this node's function? | `nodes[].role`, `nodes[].services` | being scheduled for that function |

A node that passes layer 1 but fails layer 2 is a real machine that cannot
take build jobs. A node that skips layer 1 has no identity at all.

## 2. The enrollment record

Per node, in `infra/registry.json`:

```json
{
  "id": "cynic-compute-01",
  "status": "candidate",
  "network": { "lan": "192.168.0.25" },
  "transports": { "ssh": { "target": "titouan@192.168.0.25" } },
  "enrollment": {
    "date": "2026-10-19",
    "enrolled_by": "hermes+cortex-nat",
    "review_after": "2026-11-19"
  }
}
```

Required: `id`, `status`, `network`, `transports`, `enrollment{date,
enrolled_by, review_after}`. `review_after` is mandatory — an enrollment
without an expiry is a permanent claim on the fleet.

## 3. Status semantics (unchanged, now load-bearing)

- `stub` — intent declared, nothing proven. **Base enrollment not satisfied.**
- `candidate` — the physical path is proven: planes answer their probes
  (`NETWORK_ENROLLMENT.md` §3). Base enrollment satisfied.
- `observed` — services have run there. The only status that grants an
  unattended workload.

Regression: a failed re-probe demotes the node; the demotion also withdraws
pool eligibility, because eligibility requires an enrollment that is past
`stub`.

## 4. Enforcement (implemented)

- `cynic build pool eligibility` reports `enrollment: ENROLLED / NOT
  ENROLLED` and blocks a node that is not base-enrolled, whatever its
  labels and toolchain.
- The F-gate (`.forgejo/workflows/fgate.yml`) has a `pool_eligibility` hard
  gate: a runner that is not base-enrolled, or that lacks a required tool,
  fails the gate before any compilation starts.
- `cynic federation enroll status` never qualifies `observed` without live
  probe evidence (PR #183) — the registry cannot promote a node by itself.

## 5. Target model (not yet implemented)

The friction today is manual: an operator edits the registry by hand after
wiring a machine. The target is `cynic node enroll`:

- a single-use join token (short TTL) authorizes the join;
- the node generates its keypair and probes its planes;
- the registry record is written automatically, in `candidate`;
- one human approval promotes it to `observed`;
- revocation is one command.

`docs/design/NODE_ENROLLMENT_V2.md` specifies this target. Until it exists,
the rule stays enforced by the two checks above: the registry record and
the eligibility verdict.
