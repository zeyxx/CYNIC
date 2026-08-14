# Phone Number Domain — Axiom Evaluation Criteria

Evaluate the PHONE NUMBER'S COMMUNITY-REPORTED REALITY — not the content of any single call. A telemarketer calling legitimately is not a scam. A number with coordinated false reports is not a scammer. The on-network data reveals WHO calls, HOW OFTEN, and WHETHER the behavior matches known threat patterns. This is what you score.

Judge the SIGNAL STRUCTURE encoded in reports and metadata. Community consensus reveals behavioral patterns. This is what you score.

## ASYMMETRIC CONFIDENCE

Negative assessments (BARKs) require stronger evidence than positive assessments (HOWLs) — false positives harm innocent callers. Absence of reports ≠ legitimacy.

- Reporter count <3: max axiom score = 0.45 (untested, cannot HOWL yet)
- Reporter count 3-10: max axiom score = 0.55 (building consensus)
- Reporter count >10 with agreement_rate >0.75: max axiom score = 0.618 (established consensus)

## FIDELITY
How faithful are the signals about this number? Does the reported behavior match what is claimed?

- HIGH (0.50-0.618): 10+ independent reporters with agreement_rate > 0.75, consistent labels over time, no contradictions between reported behavior and known patterns. Example: Number with 15 reporters all labeling "telemarketing", call frequency consistent with outbound sales patterns, stable label history over 30+ days.
- MEDIUM (0.25-0.45): 3-10 reporters, mixed labels, some temporal consistency. Behavior partially matches claimed category. Example: Number with 5 reporters split between "robocall" and "telemarketing" — same threat class, label ambiguity acceptable.
- LOW (0.05-0.20): Fewer than 3 reporters, contradictory labels across incompatible categories (e.g., "legitimate business" AND "scam"), or single-source data with no corroboration. Example: One report labeling a number "IRS scam" with no other reports — unverifiable.

## PHI
Is the evidence proportional and structurally sound? Do report patterns match natural behavior?

- HIGH (0.50-0.618): Report count proportional to call volume estimate, no suspicious spikes within a single short window, temporal distribution is organic (reports spread over days/weeks). Example: 20 reports accumulated over 2 weeks with steady arrival rate — consistent with organic community discovery.
- MEDIUM (0.25-0.45): Some report clustering but within normal bounds. Minor concentration from a few active reporters. Example: 8 reports where 3 came from the same day — explainable by a viral social media post about the number.
- LOW (0.05-0.20): Report spike within a single minute suggesting coordinated attack, or single reporter accounts for >80% of all reports. Example: 12 reports submitted within 60 seconds with identical text — likely coordinated false flagging.

## VERIFY
Is the judgment falsifiable and testable? Can the signals be independently corroborated?

- HIGH (0.50-0.618): Score derived from measurable signals (reporter count, agreement_rate, temporal decay), contestation mechanism exists, cross-source consistency available. Example: Number flagged on multiple independent platforms (Truecaller, community DB, carrier data) with consistent verdict.
- MEDIUM (0.25-0.45): Score is derived but some inputs are uncertain — spoofed CLI possible, low reporter confidence, single platform source. Example: Number labeled scam on one platform only, CLI may be spoofed (VOIP prefix), no carrier-level confirmation.
- LOW (0.05-0.20): Score based on a single unverified report with no corroboration path. Key claims cannot be independently tested. Example: One anonymous report claiming "bank fraud call" — no callback test possible, no cross-reference available.

## CULTURE
Does this number's behavior respect established telephony norms and community patterns?

- HIGH (0.50-0.618): Number behavior matches known legitimate or threat patterns precisely (business hours for telemarketing, rotating sequential numbers for robocall farms, specific area codes for known scam clusters). Follows established patterns for its labeled category. No impersonation of emergency services. Example: Outbound sales number calling 9am-5pm on weekdays, matching TCPA-regulated telemarketing behavior.
- MEDIUM (0.25-0.45): Behavior partially matches known patterns. Minor deviations with plausible explanation. Example: Number calling outside business hours occasionally — could be timezone mismatch or after-hours support line.
- LOW (0.05-0.20): Number impersonates emergency services (911, 112), government agencies, or known legitimate businesses. Call behavior is anomalous and doesn't fit any known legitimate or threat category. Example: Number spoofing local police department prefix with reports of demands for immediate payment — clear impersonation pattern.

## BURN
Is the judgment efficient and minimal? Is inference cost justified by signal quality?

- HIGH (0.50-0.618): Decision is clear from heuristic signals alone — reporter count, agreement_rate, and temporal distribution are sufficient. No LLM inference needed. Example: 20 reporters, 0.90 agreement_rate, consistent "robocall" label — deterministic dog can score this without inference.
- MEDIUM (0.25-0.45): Ambiguous zone where additional signals (challenge result, callback test, carrier lookup) would meaningfully change the verdict. Inference adds value here. Example: 4 reporters with 0.50 agreement_rate and mixed labels — inference on the specific call descriptions might resolve ambiguity.
- LOW (0.05-0.20): Insufficient data to score meaningfully — fewer than 3 reports with no corroborating signals. Expending inference resources produces an unreliable verdict. Example: 1 report from yesterday on an otherwise-unknown number — wait for more data rather than judge now.

## SOVEREIGNTY
Does this judgment preserve the agency of both callers and callees? Is control distributed?

- HIGH (0.50-0.618): Presumption of innocence applied, contestation mechanism available to number owner, no single entity controls the score, federated consensus used. Temporary numbers (disposable VOIP) are treated with higher suspicion by default. Example: Community-sourced score where any entity can submit counter-evidence, decaying reputation system, number owner can submit verified business identity to contest.
- MEDIUM (0.25-0.45): Score influenced by federated consensus but single-node dominance is possible. Contestation path exists but is not well-publicized. Example: Carrier-reported flagging where one major carrier's data dominates but community reports provide counterweight.
- LOW (0.05-0.20): Score from a single source with no contestation path, or a single entity can unilaterally label any number. Individual callee has no agency over incoming assessment. Example: Blacklist controlled by one private company with no appeal mechanism — sovereignty of the caller is entirely extinguished.
