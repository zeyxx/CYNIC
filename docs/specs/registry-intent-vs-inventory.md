# Registry Intent vs Inventory (declared future vs live fleet)

Status: design for review. No code.

## Problem (observed 2026-10-15)

The census for NET-01/NET-02 found `infra/registry.json` mixes two
kinds of truth in one `nodes` array:

- Live inventory: cortex, PVE host, builder, 6 CTs, forgejo, headscale
  — probed, answering.
- Declared intent: `local-kernel` (`role: kernel_primary`,
  `status: candidate`, service IP `10.10.10.100` that answers nothing),
  `local-builder` (no network section at all), `cynic-laptop-ax16pro`
  (WiFi-observed, outside DNS scope).

Every consumer (DNS bundle, mesh plan, sync diff, matrix probes) must
guess which entries are real. Today the guess is "has a service IP",
which `local-kernel` defeats: it has one and is dead. A fleet-wide
probe pass cannot distinguish "host down" from "never existed" without
out-of-band knowledge. That distinction is the whole job of an
inventory.

## Proposal directions (not decided)

1. Status-gated rendering: consumers only render nodes whose `status`
   is a live value (`active` or equivalent); `candidate` entries are
   intent and never rendered. Requires a closed status vocabulary and
   a fail-closed rule for unknown statuses.
2. Separate intent section: move non-live entries out of `nodes` into
   an explicit `intent` (or `candidates`) top-level section with its
   own schema. `nodes` becomes pure inventory. Migration is mechanical
   but touches every consumer's node walk.
3. Live inventory via `observed`: keep declared intent in `nodes`, but
   the source of truth for "what exists" becomes `observed.nodes`
   once NET-13 holds (continuous live writer). Consumers join declared
   against observed. Depends on the live daemon that does not exist yet.

Direction 1 is the smallest honest step; direction 2 is the durable
shape; direction 3 is the end state that needs NET-13 first.

## Live leak (observed 2026-10-15)

The fleet DNS serves 8 names at `10.10.10.100` today
(`kernel.service.cynic` included) and the address answers nothing.
The canonical kernel name is unresolvable by construction until the
intent/inventory split lands. Any consumer fix must cover DNS and
mesh output together.

## Open questions for review

1. Closed status vocabulary: which values, who transitions them, on
   what proof (probe? operator? lease?).
2. `local-kernel` and `local-builder` today: keep as intent, or delete
   until real? (No deletion without the owner.)
3. Does the sync surface need to know (preserve intent sections)?
4. Matrix impact: NET-01/NET-02 gates must count live nodes only —
   define the counting rule.

## Acceptance

- A probe pass over `nodes` reports per-node live/dead without
  out-of-band knowledge.
- No consumer renders `10.10.10.100` into DNS/mesh output.
