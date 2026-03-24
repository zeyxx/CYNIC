# Trading Domain — Axiom Evaluation Criteria

Evaluate the MARKET HYPOTHESIS, SIGNAL, or ANALYSIS described — not the quality of its prose. A genuine edge poorly explained is still an edge. A beautifully written thesis with no data is still noise.

Judge the SUBSTANCE: is the claim supported by data? Is the reasoning sound? Would this survive scrutiny from a quant desk?

## FIDELITY
Is this faithful to observable market data? Are the numbers verifiable? Does the claim match what the data actually shows?
- HIGH: Specific numbers cited (funding rate, OI change, volatility, correlation). Data source named. Sample size stated. Numbers are internally consistent.
- MEDIUM: Qualitative claims with some data support. Directionally correct but imprecise. Source implied but not named.
- LOW: Vague claims ("market is bearish"). Numbers that contradict each other. No data source. Assertions presented as fact without evidence.

## PHI
Is the analysis proportional? Does the strength of the claim match the strength of the evidence? Is the hypothesis well-structured?
- HIGH: Claim magnitude matches evidence magnitude. Multi-dimensional analysis (≥2 data sources). Confidence explicitly bounded. No extrapolation beyond data range.
- MEDIUM: Reasonable claim-to-evidence ratio. Single dimension but well-measured. Minor overreach.
- LOW: Extraordinary claims from ordinary data. Full-sample backtests presented as predictive. Sharpe without confidence interval. P-hacking (many tests, no correction).

## VERIFY
Is this falsifiable? Can it be proven wrong? Is the sample size sufficient? Does it survive the strongest counterargument?
- HIGH: Explicit falsification criteria stated. Walk-forward or out-of-sample validation. Statistical significance (p < 0.05). Minimum 30 observations. Counterargument addressed.
- MEDIUM: Falsifiable in principle but not tested out-of-sample. In-sample only with stated caveats. Reasonable sample size.
- LOW: Unfalsifiable ("could work in the right conditions"). Full-sample optimization. No statistical test. N < 10. Survivorship bias. No counterargument considered.

## CULTURE
Does this align with known market microstructure and economic mechanisms? Is there a reason WHY this should work?
- HIGH: Clear economic rationale (arbitrage mechanism, behavioral bias, structural flow). Consistent with academic finance literature. Mechanism explains both when it works AND when it fails.
- MEDIUM: Plausible rationale but untested. Pattern recognition without mechanism. Technical analysis with some empirical support.
- LOW: Cargo cult (SMC terms without understanding, copied from YouTube). No economic mechanism. "It worked before" without explaining why. Contradicts market efficiency without justification.

## BURN
Is this the simplest sufficient explanation? Could the same conclusion be reached with less complexity?
- HIGH: Minimal parameters. Simple hypothesis that explains the data. No overfitting indicators. Occam's razor applied.
- MEDIUM: Some unnecessary complexity but core thesis is clear. Extra parameters that don't materially change the conclusion.
- LOW: Overfit (many parameters, few data points). Complexity without explanatory power. Multiple indicators combined without justification. Signal could be replaced by a coin flip.

## SOVEREIGNTY
Does this preserve optionality? Does the analysis consider multiple outcomes? Does it avoid locking into a single narrative?
- HIGH: Multiple scenarios considered with probabilities. Risk explicitly quantified. Clear exit conditions. Acknowledges what would change the thesis.
- MEDIUM: Primary thesis with brief mention of alternatives. Some risk awareness. Position sizing implied.
- LOW: Single-narrative thinking ("BTC will definitely..."). No exit condition. All-in positioning implied. Ignores regime change possibility. Confirmation bias visible.
