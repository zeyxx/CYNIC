# Agent Messaging — addressed delivery with receipts on the coordination log

## Goal

Let any CYNIC fleet agent send an addressed message to another agent (or
role), and let every receiver know it received something — with envelope
metadata — so the receiver can judge when to open the full content.
Transport stays local-first (same host, shared log); remote fleet delivery
is explicitly out of scope for this tranche.

## Non-goals (phase 1)

- No real-time notification daemon (no long-lived listener process).
- No confidential content (log is plaintext JSONL; secrecy arrives later).
- No replacing `network notify` (watchdog alerts stay a separate path).

## Phase 2 — cross-node relay (designed now, built after phase-1 usage)

The fleet already spans nodes (hypervisor, cortex, builder, compute-01).
Phase 1 proves the envelope locally; phase 2 carries addressed entries
across nodes by reusing the existing sovereign bearer — `cynic-node`
`peer_submit_tx` (signed envelopes, SQLite projection) — with a new
message-tx kind owned by the `cynic-node` domain, not by the CLI.
The relay copies the entry into the destination node's coordination log;
receipts flow back the same way. No new transport daemon, no broker.

## A2A emergence (not adoption)

The full A2A protocol is NOT adopted. The envelope keeps A2A-shaped
fields (`messageId`, `contextId`/`in_reply_to`, parts-ready body) so a
lisière adapter can map send→`message/send` and ack→2xx/task-state later,
once emergent usage data shows which flows matter. Interop grows out of
observed traffic, not out of the spec.

## State of the art (observed, not invented)

- **A2A (Google, spec a2aproject/A2A §3.5/§4.3):** message → task; push
  notifications are best-effort webhooks; the RECEIPT is the client's
  HTTP 2xx. Lesson: separate the delivery signal (cheap, synchronous)
  from the content processing (expensive, asynchronous).
- **ai-bus (Robinho100, observed clone `~/.cynic/external/ai-bus`):**
  localhost bus + SQLite state + linear turn chain, `create_task`
  delegation. Lesson: local bus works; but ai-bus chains turns, it does
  not address messages nor acknowledge receipt — our gap is different.
- **Classical MAS theory (FIPA-ACL / KQML, blackboard systems):**
  communicative acts carry performative + envelope so the receiver can
  triage without parsing content. Lesson: envelope metadata is the
  triage instrument, not the body.
- Live paper search for 2024-2026 fleet-messaging work was attempted
  2026-10-19 and failed on tooling (search backend down); the above
  three sources are the evidenced basis. Re-run the survey before
  implementation if the operator wants full SOTA coverage.

## Decision: extend `coordination`, do not mint a domain

`cynic coordination append/read` over a JSONL log (`coordination_log.rs`,
schema `cynic.coordination.entry.v1`) already gives durable broadcast
with `kind/body/refs`. The missing pieces are addressing + receipt, both
additive optional fields. A new family would duplicate the log, the
read path, and the retention story for zero gain. Extension wins.

## Domain changes

1. `CoordinationEntry` gains optional envelope (all `#[serde(default)]`,
   backward compatible):
   - `to: Option<String>` — addressed agent id or role (`agent:<id>`,
     `role:<role>`). Absent = broadcast (current behavior).
   - `msg_id: Option<String>` — sender-generated UUID per addressed
     message. Absent = no receipt tracking.
   - `in_reply_to: Option<String>` — causal thread pointer.
   - `priority: Option<u8>` (0-3, default 1) + `summary_1line: Option<String>`
     (<= 140 chars) — the triage metadata; `read` renders these WITHOUT
     the body on request.
   - `receipt: Option<MsgReceipt>` — written ONLY by the receiver:
     `{by, ts_unix_s, state: delivered|opened}`. Sender must not set it;
     `append` rejects sender-set receipts (fail closed).
2. Pure predicates in the owning module (no I/O):
   - `admit_addressed_append(entry) -> Vec<String>` (violations;
     empty = admit): `to` present requires `msg_id` + `summary_1line`;
     receipt must be absent on append.
   - `admit_receipt(entry, by) -> Vec<String>`: receipt.state must
     advance monotonically (`delivered` → `opened`, never backward).
3. Receipts are appended as new log lines referencing `msg_id`
   (append-only log stays append-only; no in-place mutation).

## Adapter behavior

- `append_entry` / `read_entries` unchanged on disk format (JSONL, one
  entry per line). `read` gains `--to-me <id>` (addressed to me +
  broadcast) and `--unopened` (addressed with no `opened` receipt).
- No new daemon, no filesystem watcher. Polling is the receiver's
  policy, not the transport's.

## CLI arms (nest under `coordination`, zero new top-level)

- `cynic coordination send --to <id|role> --kind <k> --summary <140c>
  --body <text> [--ref ...] [--priority 0-3] [--in-reply-to <msg_id>]`
  → prints `msg_id` (generated when absent) + JSON proof.
- `cynic coordination inbox [--unopened] [--json]` → addressed-to-me +
  broadcast, summary-first projection (body only with `--full`).
- `cynic coordination ack <msg_id> --state delivered|opened`
  → appends the receipt line. Refuses backward transitions.

## Acceptance

- Unit: admit predicates (sender receipt rejected; `to` without
  `msg_id` rejected; backward receipt rejected).
- Builder: `cargo test -p cynic-cli --test coordination_*` green at the
  merge SHA; old `append/read` output byte-identical for entries
  without the new fields.
- Live-fire: two agent ids exchange send → inbox → ack on a scratch
  log; `inbox --unopened` empties after `ack opened`.

## Debt register

- D1: log is plaintext — any agent with log read access reads all
  bodies. Confidential messaging needs a sealed envelope (follow-up).
- D2: no push — receivers poll. A notify hook (`network notify`
  reuse) is a later tranche, not this one.
- D3: `msg_id` uniqueness is sender-generated UUID, not
  content-addressed; duplicates are possible, receivers dedupe by id.
- D4: paper survey (run 2026-10-19 via agy/OpenAlex+arXiv, evening session).
  Axe receipts/custody: Kadaboina 2026 (Zenodo 20644821), Alshammari &
  Bahsi 2026 (arXiv:2608.28542), Le Bihan et al. 2025 (arXiv:2507.17403),
  Liu 2026 (arXiv:2609.03425) — custody transfer + explicit delivery/read
  receipts beyond A2A push. Axe envelopes/triage: Finin et al. 1994
  (KQML, CIKM), Labrou/Finin/Peng 1999 (FIPA-ACL semantics), Ehtesham et
  al. 2025 (arXiv:2505.02279), Touheed 2026 (arXiv:2606.00962) — blind
  intent envelopes, schema-conformance triage without opening content.
  Axe append-only transport: Gelernter 1985 (Linda/blackboard), Meyer
  2023 (arXiv:2308.13836, fork/duplicate detection on shared prefix
  logs). Axe brokerless relay: custody-transfer model + Kermarrec et
  al. 2020 (gossip). Design consequences applied: sender-rejected
  receipts, summary/priority triage envelope, receipts as new lines,
  phase-2 relay over signed peer tx. Full survey text lives outside the
  repo (session output); re-run before phase 2 if flows diverge.
