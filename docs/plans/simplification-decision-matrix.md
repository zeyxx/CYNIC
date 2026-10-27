# Simplification Decision Matrix

Status: working method, not a gate. Written 2026-10-26 from a measured pass over
`cynic-host`. Every unit of code (module, port, function, crate) receives exactly
one verdict from the matrix, or a short sequence of verdicts.

## Why a matrix

An earlier draft was a linear decision tree. It broke on the first real case:
the fleet observers in `cynic-host` were unused, duplicated a live command, and
half of their private code turned out to be live through another path. The
criteria are independent of each other, so they form a matrix, and verdicts
compose.

## Axes

1. **Usage**: called (a live caller exists) or not called.
2. **Counterpart**: unique, or a duplicate of another live implementation.
3. **Fit**:
   - if called: right place and light, or wrong place (layer, crate), or heavy;
   - if not called: wanted, or not wanted. "Wanted" requires normative evidence:
     a `contracts/*.json` clause or an operator decision. Design documents do
     not count.

## Matrix

| Usage | Counterpart | Fit | Verdict |
|---|---|---|---|
| called | unique | right place, light | leave alone |
| called | unique | wrong place | **move** |
| called | unique | heavy | **simplify** |
| called | duplicate | any | **merge**: the better one wins, the other is deleted |
| not called | unique | wanted | **wire** |
| not called | unique | not wanted | **delete** |
| not called | duplicate | the dead one is poorer | **delete** |
| not called | duplicate | the dead one is better placed | **merge**: migrate the live one onto it, then delete |
| not called | duplicate | both dead | wire one, or delete both |

Verdicts compose. Merging the fleet observers toward `cynic-host` is a move of
the CLI logic followed by a deletion of the copy.

## Weight (changes order and caution, not the verdict)

Weight is the number of dependents. `cynic-protocol` has 32, a module in L4 has
none. The higher the weight, the earlier the unit is handled (bottom-up) and the
more it is verified before any change.

## Evidence rules

- A caller count comes from searching the whole workspace, tests and bins
  included, comments excluded. A grouped import (`use x::{a, b}`) defeats a
  naive path pattern: confirm by name before concluding a unit is unused.
- Check for a live half before deleting a dead half. `domain::cpu` and
  `adapters::local_cpu` looked dead next to their driver and were live through
  `adapters/local_capacity.rs`.
- File dates are unusable: history was re-rooted at `2352c0d5`, so every file
  reads as added on 2026-10-23.
- A deletion that leaves a counterpart without a producer is reported, not
  followed into a lower layer in the same change.

## Worked cases (2026-10-26)

| Case | Cell | Verdict |
|---|---|---|
| `ClaimStatus`, `Budget` duplicated in the node bus and `cynic-types` | called, duplicate | merge (done) |
| `domain -> adapters` imports in `cynic-node` | called, wrong place | move / simplify (done) |
| `fleet_{cpu,memory,health}_observer` + `remote_{cpu,memory}` in `cynic-host` | not called, duplicate of `cynic fleet observe`, poorer | delete (done, 1 008 lines) |
| `process_convergence` | not called, unique, wanted by `contracts/supervisor.json` | wire, or amend the contract: operator decision |
| `runtime_seed`, `wireguard_readback`, `plan_verifier` used only by `cynic-node` | called, wrong place | move |
| `cynic fleet observe` logic living in `cynic-cli` | called, wrong place | move down, later |

## Open consequences of the first deletion

- `cynic-protocol::host_substrate::{cpu,memory}` still define `FleetCpuReport`,
  `FleetCpuNode`, `FleetMemoryReport`, `FleetMemoryNode`. Their only producers
  were the deleted observers. Layer L0, high weight: measure consumers before
  any change.
- `domain::memory::probe::parse_free_output` was used by the deleted remote
  memory adapter and may now be unused outside tests.
- `fleet_disk_observer`, `fleet_network_observer`, `fleet_build_artefact_observer`
  are in the same family and have the same shape of verdict; each needs its own
  read.
