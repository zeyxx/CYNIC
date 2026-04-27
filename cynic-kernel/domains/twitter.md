# Twitter/Social Signal Domain — Axiom Evaluation Criteria

Evaluate the SIGNAL QUALITY of the social content — not the writing quality, not the author's popularity. A poorly written rug warning that turns out to be accurate is a HIGH-FIDELITY signal. A viral thread from a whale that contains no verifiable claim is noise.

Judge WHETHER THIS CONTENT WOULD HELP A TRADER AVOID LOSS OR FIND OPPORTUNITY. Social signals are early indicators — they precede on-chain events. Score the predictive value.

## ASYMMETRIC CONFIDENCE

Warnings (rug alerts, scam callouts) have higher potential value than hype (pump calls, "next 100x"). False negatives (missing a real rug warning) are costlier than false positives (flagging noise as signal). Score warnings generously, hype skeptically.

- Single unverified claim: max axiom score = 0.35
- Claim with on-chain evidence cited: max axiom score = 0.50
- Claim with historical accuracy (author predicted correctly before): max axiom score = 0.618

## FIDELITY
Does this content contain a TRUTHFUL, SPECIFIC signal about a token or market event?
- HIGH (0.50-0.618): Names specific token + specific mechanism (fee assignment, LP drain, freeze authority abuse). Cites verifiable evidence (tx hash, wallet address, contract analysis). Author has track record of accurate calls. Example: "@gcrtrd: $BEDROCK slow rugged — fee assignment draining wallets. Every token that assigned fees to him is dead."
- MEDIUM (0.25-0.45): Names specific token with general concern (suspicious activity, whale movement, unusual volume). No direct evidence but plausible mechanism. Author is organic with moderate history. Example: "Seeing unusual wallet activity around $TOKEN — 3 insider wallets moved 40% of supply in last hour."
- LOW (0.05-0.20): Vague claims without specific tokens or mechanisms. "Solana memes are scams" — true in aggregate but useless as signal. Emotional rather than analytical. Bot-like pattern (identical content from multiple accounts). Example: "EVERYTHING ON SOLANA IS A RUG" or "This is the next 100x gem."

## PHI
Is this content structurally coherent? Does it present a complete signal or just noise?
- HIGH (0.50-0.618): Presents claim + evidence + mechanism. Cross-references multiple data points. Thread structure with logical progression. Proportional response (alarm matches severity). Example: Multi-tweet thread documenting a scam pattern with screenshots, wallet links, and timeline.
- MEDIUM (0.25-0.45): Single coherent claim with partial evidence. Clear structure but missing verification steps. Example: Retweet with added context — "This happened to $X too, same dev wallet pattern."
- LOW (0.05-0.20): Isolated assertion without structure. All-caps hype. Copy-pasted promotional text. Engagement bait. Example: "🚀🚀🚀 $MOONTOKEN TO THE MOON 🚀🚀🚀" or identical bot ring posts.

## VERIFY
Can the CLAIMS in this content be independently verified? Are they falsifiable?
- HIGH (0.50-0.618): References on-chain data that anyone can check (mint address, tx signature, wallet address). Claims are specific enough to be proven wrong. Time-bound predictions. Example: "Dev wallet 7xK3... just pulled $200K from LP on Raydium — check the tx."
- MEDIUM (0.25-0.45): Claims are specific but verification requires effort or access (DM screenshots, private group activity, insider knowledge). Partially verifiable. Example: "KOLs are being paid $500K to promote this casino — here's the payment screenshot."
- LOW (0.05-0.20): Claims cannot be verified. Opinions presented as facts. No specific addresses, amounts, or timestamps. Unfalsifiable assertions. Example: "This team is sketch" without any specifics.

## CULTURE
Does this content follow patterns of legitimate crypto social analysis? Or does it match known scam/bot/spam patterns?
- HIGH (0.50-0.618): Follows investigative journalism norms — attribution, evidence, disclosure of position. Engages with counterarguments. Author participates in legitimate crypto community (not just shilling). Example: Security researcher thread with responsible disclosure timeline.
- MEDIUM (0.25-0.45): Standard crypto twitter analysis — opinion with some backing. Author has genuine engagement (not all RT/promo). Follows community norms but not rigorous. Example: Regular trader sharing chart analysis with risk disclaimers.
- LOW (0.05-0.20): Matches known manipulation patterns: bot ring (identical content from N accounts), recovery scammer (preys on rug victims), paid promotion without disclosure, coordinated pump campaign. Example: 17 accounts all posting "top 5 strongest memecoins on Solana" with identical token lists.

## BURN
Is this signal efficient? Does it convey maximum information with minimum noise?
- HIGH (0.50-0.618): Dense signal — every sentence adds information. No padding, no engagement bait, no unnecessary emoji spam. Data-to-noise ratio is high. Example: "Mint: X. Freeze authority active. Top wallet holds 80%. LP not locked. Age: 2h. Avoid."
- MEDIUM (0.25-0.45): Some signal buried in noise. Thread could be 60% shorter without losing information. Standard twitter verbosity. Example: Useful analysis padded with "NFA DYOR" disclaimers and personal anecdotes.
- LOW (0.05-0.20): Almost entirely noise. Engagement farming. Text-to-signal ratio is 10:1 or worse. Example: Long thread that could be summarized as "I like this token" or "crypto is bad."

## SOVEREIGNTY
Does this content preserve the reader's ability to make their own decision? Or does it manipulate?
- HIGH (0.50-0.618): Presents evidence and lets the reader decide. Discloses author's position/bias. No urgency manipulation ("BUY NOW BEFORE IT'S TOO LATE"). Empowers independent verification. Example: "Here's what I found about $TOKEN. I hold none. Check the contract yourself: [address]."
- MEDIUM (0.25-0.45): Mild opinion pushing but with enough data for independent judgment. Some urgency language but not predatory. Example: "I'm bullish on $TOKEN because of X, Y, Z — but verify the LP lock yourself."
- LOW (0.05-0.20): Active manipulation — FOMO creation, false urgency, authority exploitation ("trust me, I've been right 100 times"). Designed to short-circuit independent judgment. Recovery scammers. Pump group coordination. Example: "LAST CHANCE to get in before $TOKEN moons — insiders know, you don't."
