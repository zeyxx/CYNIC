# Governance Domain — Axiom Evaluation Criteria

Evaluate the GOVERNANCE PROPOSAL'S SOUNDNESS — not the quality of its writing, not the prestige of its author. A well-written proposal that concentrates power is dangerous. A rough proposal that distributes control and defines clear success criteria is legitimate.

Judge WHETHER THIS PROPOSAL WOULD IMPROVE THE DAO'S ABILITY TO MAKE BETTER DECISIONS OVER TIME. Governance proposals are structural changes — they encode power relationships. Score the governance design, not the marketing.

## ASYMMETRIC CONFIDENCE

Proposals that concentrate power or spend treasury without accountability have higher failure potential than those that distribute control or add reversible features. Optimistic roadmap claims without success criteria are structurally weaker than conservative scoped asks.

- Proposal with undefined success metrics: max axiom score = 0.40 (cannot evaluate outcome)
- Proposal with defined metrics but no reversibility: max axiom score = 0.50 (locked-in risk)
- Proposal with metrics + reversibility + community alignment: max axiom score = 0.618

## PROPOSAL CLASS AWARENESS

Governance proposals serve different functions. Apply appropriate criteria per class:

- **Personnel** (hire, fire, elect): Evaluate scope of authority granted, term limits, accountability mechanism, alignment with DAO mandate.
- **Treasury spend** (grant, budget, investment): Evaluate amount vs. expected value, accountability structure, reporting requirements, whether unspent funds return to treasury.
- **Parameter change** (fee, threshold, quorum): Evaluate direction of change, historical precedent, reversibility, who benefits from the change.
- **Protocol upgrade** (smart contract, governance mechanism): Evaluate audit status, migration path, emergency pause mechanism, sovereignty implications.
- **Strategic direction** (partnership, roadmap pivot): Evaluate alignment with DAO mission, resource requirements, opportunity cost, exit criteria.

## FIDELITY
Does the proposal accurately describe what it will do and what it will cost? Are the claimed benefits proportional to the ask?
- HIGH (0.50-0.618): Specific deliverables with measurable outcomes. Budget breakdown with line items. Timeline with milestones. Explicit statement of what will NOT be done. Risks acknowledged. Example: "Fund 3 months of development at $X/month. Deliverable: working prototype by date Y. Success metric: Z. If milestone missed by 30 days, remaining funds return to treasury."
- MEDIUM (0.25-0.45): General scope with plausible budget. Some deliverables defined but not fully measurable. Partial risk acknowledgment. Example: "Fund team to build feature X over 6 months. Budget: $50K. Expected improvement: significant increase in user engagement."
- LOW (0.05-0.20): Vague scope ("improve the ecosystem"), inflated budget without justification, circular reasoning ("this will make the DAO more valuable, so the DAO should fund it"), no acknowledgment of alternatives or risks. Example: "Hire a marketing team to grow the brand. Budget: $500K. This will increase token price."

## PHI
Is the proposal's governance structure harmonious? Does it include the necessary components for a sound governance action?
- HIGH (0.50-0.618): Includes rationale (why now), mechanism (exactly how), success criteria (how we know it worked), accountability (who is responsible, how are they held accountable), and exit/sunset (what happens if it fails or succeeds). Proportional ask — not overengineered, not underspecified. Example: Proposal with problem statement, proposed solution, KPIs, budget breakdown, reporting cadence, and failure condition.
- MEDIUM (0.25-0.45): Most structural components present but some missing. Accountability defined but weak (self-reported only). Success criteria present but poorly operationalized. Example: Proposal with rationale and budget but no explicit success metrics or failure conditions.
- LOW (0.05-0.20): Missing core structural components. No accountability mechanism. No success criteria. No relationship between ask and expected outcome. Example: One-paragraph proposal asking for $100K to "grow the DAO" with no further detail.

## VERIFY
Are the claims in this proposal independently verifiable? Are success metrics measurable by the community, not just the proposer?
- HIGH (0.50-0.618): Success metrics are on-chain or publicly observable (TVL, transaction count, verified code commits, published audit). Budget disbursement is tied to milestones. Third-party verification is built into the structure. Falsifiable: defines what failure looks like. Example: "Milestone 1: deploy contract to mainnet by date X — verifiable on Solana Explorer. Milestone 2: 1,000 unique wallets by date Y — verifiable via analytics."
- MEDIUM (0.25-0.45): Some metrics are verifiable, others require trusting the team. Milestone-based payment but milestones are somewhat subjective. Budget is reasonable but not granular. Example: "Monthly progress reports (community-reviewed) + final audit by external firm."
- LOW (0.05-0.20): Success cannot be measured independently. Claims are unfalsifiable ("improve reputation", "build community trust"). Self-reported metrics only. No milestone-based disbursement. Budget cannot be evaluated. Example: "This adviser will improve our governance — success is their continued engagement."

## CULTURE
Does this proposal follow the DAO's governance traditions and community norms? Does it respect established precedent?
- HIGH (0.50-0.618): Submitted through standard governance channels. Budget ask is consistent with historical precedent. Proposer has community standing or established track record. Discussion period honored. No circumvention of existing governance mechanisms. Example: Long-standing contributor submitting a properly structured proposal, consistent with past approved grants.
- MEDIUM (0.25-0.45): Mostly follows norms but with minor deviations. New contributor with limited track record but reasonable proposal. Some precedent for the type of ask. Example: New team member requesting funding for a novel project type — reasonable but without established precedent.
- LOW (0.05-0.20): Circumvents established governance (fast-tracked without proper discussion, emergency mechanism abused for non-emergency). Proposer conflict of interest not disclosed. Amount significantly above historical norms without justification. Creates precedent that could be abused. Example: Team member proposing to pay themselves retroactively for work done before the grant was approved.

## BURN
Is the resource ask proportional to the expected value? Is the governance overhead minimal relative to the governance gain?
- HIGH (0.50-0.618): Budget is justified line-by-line. No unnecessary intermediaries. Expected ROI is plausible given the ask. Scope is minimal for the goal — not gold-plated. Treasury risk is bounded (staged disbursement, capped exposure). Example: $30K for a 3-month audit of a $10M protocol — proportional risk management.
- MEDIUM (0.25-0.45): Budget seems reasonable but not fully justified. Some inefficiency in structure. ROI plausible but optimistic. Treasury exposure manageable. Example: $200K for a 12-month business development initiative with estimated $1M in new partnerships — plausible but unverifiable.
- LOW (0.05-0.20): Budget is inflated or unjustified. Ask is disproportionate to expected value. Scope creep embedded in the proposal. Treasury exposure is unbounded. Example: $1M "ecosystem fund" with no specified deployment criteria or return conditions.

## SOVEREIGNTY
Does this proposal preserve the DAO's ability to make independent decisions in the future? Does it avoid concentrating power or creating lock-in?
- HIGH (0.50-0.618): Proposal includes sunset clause or renewal requirement. Authority granted is scoped and time-limited. Reversible — can be undone by future governance vote. Does not create dependencies on a single party. Distributes decision-making or resources rather than concentrating them. Example: 6-month advisory role with explicit renewal vote, no exclusive partnership, defined authority scope.
- MEDIUM (0.25-0.45): Some lock-in risk but manageable. Authority is scoped but not time-limited. Creates a new dependency but with exit provisions. Example: Annual advisory contract with termination clause — some lock-in but with clear exit mechanism.
- LOW (0.05-0.20): Creates permanent or hard-to-reverse dependencies. Concentrates authority in a single person or small group without term limits. Establishes exclusive partnerships that reduce the DAO's future options. No sunset clause. Grants authority that exceeds what the proposal requires. Example: Permanent appointment of a central authority with no removal mechanism, or exclusive 5-year contract with a single vendor.

## PRIOR CALIBRATION

- **Treasury spend proposals**: Most underpromise and overspend. Default prior: mild skepticism. Threshold to HOWL: specific milestones + milestone-based disbursement + success metrics the community can measure.
- **Personnel proposals**: Most governance issues trace back to unclear accountability. Default prior: neutral-skeptical. Threshold to HOWL: scoped authority + defined term + removal mechanism.
- **Parameter changes**: Direction matters more than magnitude. Default prior: evaluate direction, not precision. Threshold to HOWL: historical data justifying the change + reversibility path.
- **Emergency proposals**: Legitimate emergencies are rare; emergency mechanism abuse is common. Default prior: high skepticism. Scrutinize the urgency claim first. If urgency is contrived, BARK regardless of substance.

When claims conflict (ambitious roadmap vs. modest budget), trust the budget as the real constraint — ambitious goals with modest budgets signal either dishonesty or naivety.
