# CI Evolution — Suite Survivability and the Five Adoption Metrics

Status: Active proposal, implemented in `.github/workflows/ci.yml` (this PR).
Authority: observed runtime state of the sovereign CI provider, read through the
provider's own API. No metric is claimed without a collection command that can
reproduce it.

---

## 1. Problem

The verification job is one sequential job, six steps, fail-fast. When the first
test suite fails, every later suite is **skipped**, and the run reports one red
dot that names no other suite. Two consequences:

- A run tells you the *first* failure, never the *whole* failure set. The other
  suites are unknown, not green.
- Fixing the first failure and re-running costs a full pipeline cycle before the
  second failure becomes visible. Diagnosis is serialized by accident.

## 2. Change

`.github/workflows/ci.yml` now runs every test suite to completion in a single
job, then emits **one** verdict.

- Each of the four test suite steps gets `id:` and `continue-on-error: true`:
  - `test_core_types` — `cargo test -p cynic-types`
  - `test_knowledge` — `cargo test -p cynic-knowledge`
  - `test_inference` — `cargo test -p cynic-inference`
  - `test_canonical_paths` — `cargo test -p cynic-cli --test canonical_paths_coherence`
- A final step `Aggregate Test Suite Verdict` runs with `if: always()` and is the
  **only** authority for the job's outcome. It fails the job when any executed
  suite failed, and also when a hard gate (JSON contract validation, workspace
  compilation) did not succeed — because a broken hard gate means the suites
  never all executed, which is a failed run and not a green one.

The two hard gates (`validate_json`, `check_compile`) deliberately keep normal
failure semantics: a contract that does not parse is not a test result to
aggregate, it is a refusal to run.

### 2.1 Why the verdict reads `outcome`, not `conclusion`

With step-level `continue-on-error: true`, the step's `conclusion` is rewritten
to `success` while its `outcome` keeps the raw result. Reading `conclusion`
would make the aggregate blind to every failure it exists to catch. The
aggregate therefore reads `steps.<id>.outcome`.

### 2.2 Consequence for readers

A suite step showing **green** is not proof. It only means "this suite did not
stop the job". The verdict is the job's `conclusion`; the aggregate log line
`Aggregate verdict: PASS|FAIL` is the single readable statement of the run.

The job is **not renamed**. Renaming it (or the workflow) would silently orphan
any branch protection rule matching the old check context.

## 3. The Five Metrics

All five are computed from the provider's own run records — nothing is inferred
from local state. `BASE` is the provider API root, `OWNER`/`REPO` the canonical
repository coordinates, `TOKEN` the CI read token.

```bash
BASE="https://forgejo.synapse.cynic/api/v1"
OWNER="jeanterre13"; REPO="CYNIC"
TOKEN="$(cat /etc/cynic/secrets/forgejo-token)"
AUTH=(-H "Authorization: token ${TOKEN}")
```

Every command below is reproducible by an operator; each metric names the owner
responsible for acting on it.

### 3.1 Main green rate

- **Definition**: share of workflow runs on `main` that concluded `success`.
- **Formula**: `successful_runs / total_runs` over the trailing window (30 days).
- **Collection**:
  ```bash
  curl -fsS "${AUTH[@]}" "${BASE}/repos/${OWNER}/${REPO}/actions/tasks?limit=200" \
    | jq '[.[] | select((.head_branch // .commit.branch // "") == "main")]
           | {total: length,
              green: (map(select(.status == "success")) | length),
              green_rate: ((map(select(.status == "success")) | length) as $g
                            | (length) as $n | if $n == 0 then null else ($g / $n) end)}'
  ```
- **Target**: >= 0.95 over 30 days.
- **Owner**: CI steward. Review cadence: weekly.
- **Action on breach**: a green rate below target means the branch is not
  trustworthy; stop merging and fix the pipeline before adding code.

### 3.2 Mean-time-to-green (MTTG)

- **Definition**: mean wall-clock duration of **successful** runs on `main`,
  from run creation to conclusion.
- **Formula**: `sum(duration) / count(successful_runs)`, duration in seconds.
- **Collection**:
  ```bash
  curl -fsS "${AUTH[@]}" "${BASE}/repos/${OWNER}/${REPO}/actions/tasks?limit=200" \
    | jq '[.[] | select(.status == "success" and ((.head_branch // .commit.branch // "") == "main"))]
           | map(.elapsed_seconds // ((.finished_at | fromdateiso8601)
                                      - (.started_at | fromdateiso8601)))
           | {count: length,
              mttg_seconds: (if length == 0 then null
                              else ((add / length) | floor) end)}'
  ```
- **Target**: stable around the measured baseline; investigate any regression
  above +25% of baseline.
- **Owner**: CI steward. Review cadence: weekly, with the baseline in the run
  report.
- **Why it matters**: MTTG is the cost of learning that a change is wrong. It is
  the metric the aggregate verdict is meant to *reduce* — one run now reports
  the whole failure set instead of serializing discovery across cycles.

### 3.3 Flaky rate

- **Definition**: share of runs that failed and were then re-run on the **same
  HEAD SHA**, which passed.
- **Formula**: `flaky_runs / total_runs`, where a run is flaky when a later run
  exists for the same `head_sha` with `success` while this run concluded
  `failure` and was superseded by it.
- **Collection**:
  ```bash
  curl -fsS "${AUTH[@]}" "${BASE}/repos/${OWNER}/${REPO}/actions/tasks?limit=200" \
    | jq 'group_by(.head_sha)
           | map(sort_by(.created_at))
           | map(select(length > 1
                        and (.[0].status == "failure")
                        and (map(.status) | index("success"))))
           | {total_runs: (map(length) | add),
              flaky_groups: length,
              flaky_rate: (if (map(length) | add) == 0 then null
                           else (length / (map(length) | add)) end)}'
  ```
- **Target**: < 0.05. Any non-zero flake on a **deterministic** suite is a
  defect to fix, not noise to tolerate.
- **Owner**: the suite's crate owner.
- **Discipline**: **no automatic retry.** A retry is a human or agent decision,
  and every retry is a data point in this metric (see `BUILD_LIFECYCLE.md`,
  D1). A metric that quietly retries would hide the very flake it measures.

### 3.4 Build duration drift

- **Definition**: per-run duration against a rolling baseline (median of the last
  20 successful runs on `main`).
- **Formula**: `drift_pct = (duration - baseline) / baseline * 100`.
- **Collection**:
  ```bash
  curl -fsS "${AUTH[@]}" "${BASE}/repos/${OWNER}/${REPO}/actions/tasks?limit=200" \
    | jq '[.[] | select(.status == "success" and ((.head_branch // .commit.branch // "") == "main"))]
           | map(.elapsed_seconds) | sort | .[-20:]
           | {baseline_seconds: ((add / length) | floor),
              last_seconds: .[-1],
              drift_pct: (if (add / length) == 0 then null
                          else (((.[-1] - (add / length)) / (add / length)) * 100 | floor) end)}'
  ```
- **Target**: |drift| <= 20% against baseline; a hard ceiling of 2x baseline is a
  stop-the-line signal.
- **Owner**: CI steward. Review cadence: per merge, alert on breach.
- **Why it matters**: duration drift is the earliest symptom of a leaking cache,
  an unbounded dependency graph, or a suite that grew a hidden sequential wait.
  Green rate stays 1.00 while the pipeline silently doubles in cost.

### 3.5 Most-failing suite

- **Definition**: rank the four suites by failure count over the window; expose
  the worst and its share.
- **Formula**: `suite_failures[suite] / total_suite_failures`.
- **Collection**: the aggregate step's `outcome` per suite is already the raw
  signal; read it from the run's step records.
  ```bash
  curl -fsS "${AUTH[@]}" "${BASE}/repos/${OWNER}/${REPO}/actions/tasks?limit=200" \
    | jq -r '.[] | select((.head_branch // .commit.branch // "") == "main") | .id' | while read -r run; do
        curl -fsS "${AUTH[@]}" "${BASE}/repos/${OWNER}/${REPO}/actions/runs/${run}/jobs"
      done | jq -s '[.[][] | .steps[]? | select(.conclusion == "failure")
                              | select(.name | test("Test Suite|Coherence"))
                              | .name] | group_by(.)
          | map({suite: .[0], failures: length}) | sort_by(-.failures)
          | {worst: .[0], ranking: .}'
  ```
- **Target**: the worst suite's share should fall over time; a single suite
  holding > 50% of failures for two consecutive weeks is a named work item.
- **Owner**: the suite's crate owner.
- **Why it matters**: with fail-fast the other three suites were invisible.
  Ranking them is only possible because all four now run.

## 4. Manual branch-protection step

The pipeline change alone does not protect `main`. The required check must be
set **by hand** on the provider, and it must match the check context name
**exactly**.

**Context name to require**: `Layer 0-4 Verification Gate`

This is the job's `name:`, not its id `verify`. The check context for a job with
a custom name is the display name. The aggregate step does not add a new check
context — it changes when the existing one goes red. **Do not rename the job**,
or the rule stops matching and every PR sits blocked with a green pipeline.

Forgejo (Gitea-compatible API), applied to `main`:

```bash
curl -fsS -X POST "${AUTH[@]}" \
  -H "Content-Type: application/json" \
  "${BASE}/repos/${OWNER}/${REPO}/branch_protections" \
  --data '{
    "name": "main",
    "enable_push": "all",
    "enable_status_check": true,
    "status_check_contexts": ["Layer 0-4 Verification Gate"]
  }'
```

Verify the rule read-back before calling it done:

```bash
curl -fsS "${AUTH[@]}" "${BASE}/repos/${OWNER}/${REPO}/branch_protections/main"
```

The read-back must show `enable_status_check: true` and the context list
containing `Layer 0-4 Verification Gate`.

Two operational notes:

- Because suite steps are `continue-on-error`, **never** require per-suite check
  contexts — they are not emitted as separate checks, and a green suite step is
  not evidence anyway.
- After any rename of the job or workflow, update
  `status_check_contexts` in the **same change**; a rule that matches nothing
  blocks merges silently.

## 5. Honest limits

- The metrics are computed from provider run records and depend on the provider
  API shape (`actions/tasks`, `actions/runs/{id}/jobs`). If the provider
  changes its API, the collection commands — not the definitions — change.
- Run records expose the branch under `head_branch` on Gitea-compatible
  providers and under `commit.branch` elsewhere; every collection command
  resolves it tolerantly as `.head_branch // .commit.branch // ""` instead of
  assuming one spelling. A record that carries neither is simply not on `main`
  as far as the metric is concerned.
- `flaky_rate` counts *observed* retries. It cannot see a flake that was never
  retried, and the no-auto-retry discipline is what keeps the measurement
  honest.
- The aggregate verdict is one gate for the whole job. It does not replace
  per-suite evidence: a red suite still leaves its own log, and its owner still
  has to read it.
- None of the five metrics is wired into CI yet. They are operator queries until
  a scheduled collection surface exists; this document is the specification for
  that surface.
