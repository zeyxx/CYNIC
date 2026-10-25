# Kyon TCC Vertical — Policy Document

*Version 0.1 — 2026-10-25. Draft for review.*

## 1. Mission

Kyon is a sovereign cognitive membrane for the person. This vertical applies
TCC-inspired (Cognitive Behavioral Therapy) principles to help the person
observe, anchor, and measure their own actions over time. It does not diagnose,
prescribe, or replace a clinician. It complements real support (psychologist,
psychiatrist, peer network) and offers a sovereign alternative for those
without access.

## 2. What Kyon does

1. **Observes** — records events (sport, meditation, nutrition, sobriety,
   project materialization) as local-first observations with provenance.
2. **Anchors** — timestamps every observation so the person recovers time
   consciousness and reduces marked amnesia.
3. **Measures** — computes a general return at instant T from observed
   events (sport minutes, meditation minutes, nutrition count, projects
   materialized, sobriety days).
4. **Proposes reversible experiments** — N=1 micro-experiments chosen by
   the person (e.g., "cut notifications for 3 days, observe frequency
   change"). Kyon never blocks an action; it creates conditions for conscious
   decision.
5. **Preserves continuity** — a policy chosen yesterday is an engagement
   with oneself, not a prison. Changing it requires explicit reconfirmation.

## 3. What Kyon does NOT do

1. **Does not diagnose** — no clinical label is ever assigned.
2. **Does not prescribe** — no medication, no treatment plan, no dosage.
3. **Does not replace a clinician** — Kyon complements, never substitutes.
4. **Does not block** — Kyon warns, slows, illuminates. It never prevents
   the person from acting.
5. **Does not gamify with dopamine** — no streaks, no badges, no
   leaderboards. The general return is a mirror, not a score to chase.

## 4. Medical disclaimer

> Kyon is not a medical device. It is not certified under MDR (EU) or FDA
> (US). It does not provide diagnosis, treatment, or medical advice. If you
> experience a medical emergency, suicidal ideation, or severe distress,
> contact emergency services or your clinician immediately. Kyon is a
> measurement and anchoring tool — it helps you see patterns, not replace
> professional care.

## 5. Epistemic framing (TCC-aligned)

Every observation carries an epistemic class (from KYON-HUMAN-MODEL):

| Class | Meaning | Kyon usage |
|---|---|---|
| Observed | Local measurement with provenance | Show trend |
| SelfReported | Person's own declaration | Show as declared |
| ExperimentalPersonal | N=1 consented experiment result | Update personal baseline |
| Scientific | Sourced scientific model | Contextualize, never conclude |
| Clinical | Qualified professional interpretation | Only with consent and mandate |
| Unknown | No or conflicting explanation | Preserve openness |

**Rule:** Sobriety observations (alcohol, tobacco, cannabis, hard drugs) are
always `SelfReported` or `Observed`. They are never `Clinical`. Kyon never
says "you are addicted" — it shows "you declared X, here is the trend over
time."

## 6. TCC principles applied

1. **Self-monitoring** (Mirror) — record events, track triggers and
   consequences. No judgment, just data.
2. **Cognitive restructuring** (Shield) — when a pattern is detected, Kyon
   presents alternative interpretations (hypothesis A vs B) and lets the
   person decide which fits.
3. **Behavioral experiments** (Askesis) — reversible N=1 experiments with
   pre/post measurement. E.g., "no alcohol for 7 days, observe sleep
   quality and mood."
4. **Relapse prevention** — milestones (1, 3, 7, 30, 90, 180 days) are
   markers, not achievements. A reset is data, not failure.
5. **Homework between sessions** — the person logs daily; Kyon shows the
   week's pattern before the next psy session.

## 7. Gamification of life (General Return)

The `GeneralReturn` is a computed view at instant T:

- `sport_minutes` — total sport duration in window
- `meditation_minutes` — total meditation duration in window
- `nutrition_score` — count of nutrition events in window
- `sobriety_days` — days since last use (per substance)
- `projects_materialized` — count of projects that shipped
- `total_events` — overall activity volume

**Rule:** The general return is a mirror, not a judge. It shows "here is
what you did" — it never says "you should have done more." The person
decides what matters.

Metrics are stored in an extensible map (BTreeMap<String, f64>). Standard
keys are defined (sport_minutes, meditation_minutes, nutrition_count,
projects_materialized, total_events). Divergent profiles add their own keys
(e.g., stimming_minutes, routine_adherence) without modifying the code.

## 8. Data model (derived from policy)

An observation is a universal `Observation` (cynic-protocol) with:

- `domain = "personal-anchoring"`
- `tool = "sport" | "meditation" | "nutrition" | "sobriety" | "project_materialized"`
- `target = specific action (e.g., "run", "sit", "meal", "release")`
- `timestamp` = event time (ms)
- `value` = JSON with domain-specific data (duration_ms, substance, score, etc.)
- `context` = free-form JSON (intensity, mood, notes)
- `status = "observed"` (default)

Milestones (sobriety) are computed, not stored: `SobrietyMilestone::highest_reached(days_sober)`.

Outcomes (decision→result) are stored separately as `AnchorOutcome`.

## 9. Storage and sync

- **Local-first:** every observation is written to local storage first
  (Room on mobile, SQLite on desktop). Network is never in the critical path.
- **Sync:** pending observations are pushed to cynic-node via POST /observe
  every 15 minutes when network is available.
- **No cloud, no account, no external server.**
- **WAL:** observations are appended to the CYNIC WAL (cynic-storage) for
  integrity and replay.

## 10. Limits and reconfirmation

- A policy chosen on day 1 can be changed on day 30, but Kyon will:
  1. Recall the original intention and context.
  2. Show the observed gap and consequences.
  3. Ask for explicit confirmation.
  4. Record the new decision separately.
  5. Observe the effects of the change.
- No silent policy mutation.

## 11. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Kyon becomes a diagnostic tool | Disclaimers + epistemic classes + never Clinical |
| Gamification creates dopamine loop | No streaks/badges; mirror only |
| Person becomes dependent on Kyon | Periods without observation allowed; Kyon can be silent |
| Monitoring becomes rumination | Frequency caps; Kyon knows when to stop |
| Sobriety data leaked | Local-first; ZK for federation; no central DB |

## 12. Relationship to CYNIC

This vertical is a brick of the "cognitive sovereign OS" facet of CYNIC.
It reuses:
- `cynic-protocol::observation_record::Observation` (universal primitive)
- `cynic-storage` (WAL + projections + ports)
- `cynic-node` (HTTP /observe route)
- `cynic-anchoring` (business logic: milestones, general return, outcomes)

It does not reuse:
- `cynic-finance` (finance domain)
- `cynic-askasis` (metaphor, needs review)
- `cynic-consensus` / `cynic-network` (node coordination, not personal data)
