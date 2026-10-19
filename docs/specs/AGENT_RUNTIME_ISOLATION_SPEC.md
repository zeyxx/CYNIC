# Agent Runtime Isolation & Agent Factory Spec (DRAFT — parked for dedicated session)

**Schema:** `cynic.spec.agent_runtime_isolation.v1`
**Status:** Draft, not normative. Parked here so the RAG session stays focused.
**Owner session:** dedicated follow-up (this file is intentionally uncommitted).

---

## I. Observed substrate facts (measured 2026-10-15, not assumed)

- cortex workstation: 13 859 MB total, ~12 856 MB used, ~1 002 MB available, 0 swap.
  Multiple Hermes-Agent Python processes share the box with the daily driver
  workload (browser, IDE, embedding runner). Python agents are RAM-heavy by
  construction; the box multi-tasks and the margin is gone.
- compute-01 (192.168.0.25): 15 369 MB total, ~14 139 MB available. Headroom
  is real there, unlike cortex.
- Embedding slot law (measured, falsified the 2 048-token assumption):
  llama-server enforces `n_ctx / --parallel` tokens per input. cortex
  `-c 4096 --parallel 2` and compute-01 `-c 4096 --parallel 2` now give
  2 048-token slots (corpus max: 1 101 tokens, margin 1.86x).
- lxc106 (10.10.10.106:8080) is admin-unreachable: ssh as `cynic`, `titouan`,
  `user` all denied (publickey,password). Its flags, ctx ceiling and health
  are unmeasurable from here. It was therefore excluded from the calibration
  cluster. This is a Proxmox/hypervisor-plane problem, owned by CYNIC
  (`cynic-lxc`, `cynic-proxmox`), not by the RAG bench.

## II. Problem

Personal workstation doubles as agent host, inference host, and daily driver.
No isolation boundary exists between operator workload and agent workload, so
every new agent (or runner slot) directly taxes interactive latency and risks
OOM on a swapless box. This does not scale to an agent factory.

## III. Direction (to be designed in the dedicated session)

Run agent instances inside containers/VMs, isolated from the personal
workstation, attached over tmux/ssh (or other transports to be evaluated:
mosh for roaming, Eternal Terminal, plain ssh+tmux as baseline; Vsock vs
bridged networking for local VMs; LXC vs microVM/Kata for the isolation
strength question). One industrialized pattern, many uses: dev sandboxes,
build runners, embedding hosts, agent workers — the agent factory.

## IV. Constraints for the dedicated-session plan

1. Organic execution: observe substrate first (RAM/CPU/IO roofs per host),
   then isolate, then consolidate against edge cases as data returns.
2. cortex stays a thin operator endpoint: runners and agents migrate out,
   the workstation keeps interactive priority with a hard memory ceiling.
3. lxc106 recovery (access, flags audit, ceiling registry) rides the same
   Proxmox/hypervisor workstream, via CYNIC crates, not ad-hoc ssh.
4. Dynamic registries over hardcode: host capacities, runner ceilings
   (`n_ctx`, `--parallel`, slot tokens), and agent placements belong in
   `infra/` JSON contracts, never in shell flags or prose.
5. Projection plurality: `cynic agent` is ONE projection among others
   (MCP, REST, daemon) over the proxmox/lxc/sandbox domain crates, which
   own all operational logic behind ports. No domain logic lives in any
   projection.

## V. Pending micro-action (RAG session, post-calibration only)

compute-01 has 14 GB free: there was no reason to halve its parallelism.
After the running calibration completes, restore
`-c 8192 --parallel 4` (still 2 048-token slots, 4 workers back) and
re-probe with the 1 101-token chunk. Do NOT restart it mid-run: the live
bench uses fail-closed batching and a restart could poison in-flight lots.

## VI. Falsifiers

- An agent host is admitted only if `available_RAM >= agents + runners + OS + margin`
  holds under measured peak, not idle, load.
- Any placement decision without a prior substrate measurement is rejected.
- lxc106 rejoins a cluster only after its ceiling is measured, not assumed.
