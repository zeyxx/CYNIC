# Social Signal Domain — Axiom Evaluation Criteria
# (slug: twitter — backward-compat with existing crystals)

Evaluate the SIGNAL QUALITY of **human-authored** social content from Twitter, Telegram, and similar platforms — not the writing quality, not the author's popularity. A poorly written rug warning that turns out to be accurate is a HIGH-FIDELITY signal. A viral post from a whale that contains no verifiable claim is noise.

Judge WHETHER THIS CONTENT WOULD HELP A TRADER AVOID LOSS OR FIND OPPORTUNITY. Social signals are early indicators — they precede on-chain events. Score the predictive value.

## SCOPE — What belongs in this domain

This domain evaluates **human analytical or opinion content**: rug alerts, token calls, market analysis, project warnings, trade rationale.

**Route elsewhere or drop before reaching this domain:**
- Automated buy-bot outputs (structured price/tx data, e.g. SOL TRENDING format) → extract mint → `token-analysis`
- Raid coordination messages ("Raid Ended — Targets Reached") → drop, structural BARK
- Shill service promotions ("We do Twitter raids, telegram shilling…") → drop, structural BARK
- Pure news headlines with no analysis → low-signal, score accordingly

## ASYMMETRIC CONFIDENCE

Warnings (rug alerts, scam callouts) have higher potential value than hype (pump calls, "next 100x"). False negatives (missing a real rug warning) are costlier than false positives (flagging noise as signal). Score warnings generously, hype skeptically.

- Single unsubstantiated claim or opinion: max axiom score = 0.35
- Claim with verifiable evidence cited (on-chain, reputable source, or logical deduction): max axiom score = 0.50
- Claim with historical accuracy (author predicted correctly before): max axiom score = 0.618

## FIDELITY
Does this content contain a TRUTHFUL, SPECIFIC signal about a token or market event?
- HIGH (0.50-0.618): Names specific token + specific mechanism (fee assignment, LP drain, freeze authority abuse). Cites verifiable evidence (tx hash, wallet address, chart, reputable report). Author has track record of accurate calls. Example: "@gcrtrd: $BEDROCK slow rugged — fee assignment draining wallets. Every token that assigned fees to him is dead." / Telegram: "This dev wallet just migrated LP to a fresh address — here's the tx."
- MEDIUM (0.25-0.45): Names specific token with general concern (suspicious activity, whale movement, unusual volume). No direct evidence but plausible mechanism. Author is organic with moderate history. Example: "Seeing unusual wallet activity around $TOKEN — 3 insider wallets moved 40% of supply in last hour."
- LOW (0.05-0.20): Vague claims without specific tokens or mechanisms. "Solana memes are scams" — true in aggregate but useless as signal. Emotional rather than analytical. Example: "EVERYTHING ON SOLANA IS A RUG" or "This is the next 100x gem." Telegram: project promo without any verifiable claims.

## PHI
Is this content structurally coherent? Does it present a complete signal or just noise?
- HIGH (0.50-0.618): Presents claim + evidence + mechanism. Cross-references multiple data points. Logical progression. Proportional response (alarm matches severity). Example: Thread documenting a scam pattern with wallet links and timeline. / Telegram: Detailed analysis post with chart screenshot, specific price levels, and invalidation condition.
- MEDIUM (0.25-0.45): Single coherent claim with partial evidence. Clear structure but missing verification steps. Example: "This happened to $X too, same dev wallet pattern."
- LOW (0.05-0.20): Isolated assertion without structure. All-caps hype. Copy-pasted promotional text. Engagement bait. Example: "🚀🚀🚀 $MOONTOKEN TO THE MOON 🚀🚀🚀" or Telegram group spam with identical content from multiple accounts.

## VERIFY
Can the CLAIMS in this content be independently verified? Are they falsifiable?
- HIGH (0.50-0.618): References data anyone can check — on-chain (mint address, tx signature, wallet), public charts, documented price history, or reputable external reports. Claims are specific enough to be proven wrong. Time-bound or condition-bound predictions. Example: "Dev wallet 7xK3... just pulled $200K from LP on Raydium — check the tx." / Telegram: "BTC testing 48k support — if it closes below, next target is 44k."
- MEDIUM (0.25-0.45): Claims are specific but verification requires effort or access (screenshots, insider knowledge, private data). Partially verifiable. Logical deduction from observable facts without direct citation.
- LOW (0.05-0.20): Claims cannot be verified. Opinions presented as facts. No specific addresses, amounts, timestamps, or sources. Unfalsifiable assertions. Example: "This team is sketch" without any specifics.

## CULTURE
Does this content reflect analytical integrity? Or does it match known manipulation patterns?
- HIGH (0.50-0.618): Discloses position or bias. Acknowledges uncertainty. Engages with counterarguments or presents evidence for the reader to judge. Author participates in legitimate crypto community beyond just shilling. Example: "Here's what I found — I hold none of this token. Verify yourself." / Telegram: Trade diary entry with explicit entry, stop-loss, and reasoning.
- MEDIUM (0.25-0.45): Honest opinion with some supporting data. Follows community norms, not rigorous but genuine. Example: Regular trader sharing analysis with risk disclaimers.
- LOW (0.05-0.20): Matches known manipulation patterns: coordinated pump campaign, paid promotion without disclosure, recovery scammer preying on rug victims, raid service offering, identical content from multiple accounts. Telegram-specific patterns: "We do Twitter raids / telegram shilling / upvotes — DM @X", raid bot results, unsolicited token promotions with no substance.

## BURN
Is this signal efficient? Does it convey maximum information with minimum noise?
- HIGH (0.50-0.618): Dense signal — every sentence adds information. No padding, no engagement bait, no unnecessary emoji spam. Data-to-noise ratio is high. Example: "Mint: X. Freeze authority active. Top wallet holds 80%. LP not locked. Age: 2h. Avoid."
- MEDIUM (0.25-0.45): Some signal buried in noise. Post could be 60% shorter without losing information. Useful analysis padded with "NFA DYOR" disclaimers and personal anecdotes.
- LOW (0.05-0.20): Almost entirely noise. Engagement farming. Text-to-signal ratio is 10:1 or worse. Example: Long post that could be summarized as "I like this token" or "crypto is bad." Telegram: project launch announcement that is 80% emoji, 20% fact.

## SOVEREIGNTY
Does this content preserve the reader's ability to make their own decision? Or does it manipulate?
- HIGH (0.50-0.618): Presents evidence and lets the reader decide. Discloses author's position/bias. No urgency manipulation ("BUY NOW BEFORE IT'S TOO LATE"). Empowers independent verification. Example: "Here's what I found about $TOKEN. I hold none. Check the contract yourself: [address]."
- MEDIUM (0.25-0.45): Mild opinion pushing but with enough data for independent judgment. Some urgency language but not predatory. Example: "I'm bullish on $TOKEN because of X, Y, Z — but verify the LP lock yourself."
- LOW (0.05-0.20): Active manipulation — FOMO creation, false urgency, authority exploitation ("trust me, I've been right 100 times"). Designed to short-circuit independent judgment. Recovery scammers. Pump group coordination. Telegram: "LAST CHANCE — insiders know, you don't. Buy before it 10x tonight."
