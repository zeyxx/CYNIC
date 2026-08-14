# Chess Domain — Axiom Evaluation Criteria

Evaluate the chess MOVE, STRATEGY, or POSITION described — not the quality of its textual description. A brilliant sacrifice poorly described is still brilliant chess. A blunder described eloquently is still a blunder.

## FIDELITY
Is this faithful to sound chess principles? Does it follow established opening theory, sound positional concepts, or correct tactical patterns?
- HIGH (0.55-0.65): Mainline theory, proven by decades of grandmaster practice. Strong strategic logic with concrete tactical backing. Example: The Ruy Lopez (1.e4 e5 2.Nf3 Nc6 3.Bb5) — centuries of GM practice.
- MEDIUM (0.25-0.45): Playable sideline or reasonable deviation. Not refuted but not the strongest continuation. Example: The Bird Opening (1.f4) — playable but not mainstream.
- LOW (0.05-0.20): Violates fundamental principles (premature queen development, neglecting center, ignoring king safety). Known refuted lines. Example: 1.e4 e5 2.Qh5 — premature queen out, easily punished.

## PHI
Is this structurally harmonious? Are the pieces coordinated? Is the pawn structure sound? Is the plan proportional to the position's demands?
- HIGH (0.55-0.65): Pieces work together, pawns support the plan, no wasted moves. The position flows logically. Example: The Hedgehog (…a6/b6/d6/e6) — small center, all pieces coordinate behind it for a harmonious counterattack.
- MEDIUM (0.25-0.45): Some coordination but minor inefficiencies. Structure is acceptable. Example: The Stonewall Dutch (…f5/e6/d5/c6) — solid but the dark-square bishop is structurally bad.
- LOW (0.05-0.20): Pieces uncoordinated, pawn structure compromised without compensation. Chaotic with no plan. Example: Playing 1.e4 d5 2.exd5 Qxd5 3.Nc3 Qa5 4.d4 e5?! — scattered pieces, no structural coherence.

## VERIFY
Does this SURVIVE concrete analysis? When tested against the strongest replies, does the strategy hold?
- HIGH (0.55-0.65): Engine-verified or demonstrated across thousands of master games. Survives the strongest counterplay. Example: The Berlin Defense (3…Nf6 in the Ruy Lopez) — tested at world championship level, uncracked.
- MEDIUM (0.25-0.45): Sound in principle but vulnerable to specific refutations. Requires precise play to maintain. Example: The King's Gambit (1.e4 e5 2.f4) — creative and forceful but Black has concrete equalizing lines.
- LOW (0.05-0.20): Easily refuted. One accurate reply demolishes the entire idea. Does not survive scrutiny. Example: The Scholar's Mate attempt (2.Bc4/3.Qh5/4.Qxf7) — trivially refuted by 2…Nc6 3…g6.

## CULTURE
Does this honor chess tradition and the established body of opening theory, endgame tablebases, and strategic principles developed over centuries?
- HIGH (0.55-0.65): Classical principles respected. Builds on the accumulated wisdom of chess history. Example: The Queen's Gambit Declined (1.d4 d5 2.c4 e6) — the classical central defense, centuries of theory.
- MEDIUM (0.25-0.45): Deliberately breaks convention for concrete reasons (hypermodern ideas, calculated provocations). Example: The Alekhine Defense (1.e4 Nf6) — hypermodern provocation, invites overextension with concrete logic.
- LOW (0.05-0.20): Ignores established knowledge without justification. Reinvents the wheel poorly. Example: 1.g4 (Grob Attack) — flouts classical development principles with no compensating idea.

## BURN
Is this efficient? Are moves purposeful? Could any move be removed without weakening the position?
- HIGH (0.55-0.65): Every move has a clear purpose. No wasted tempi. Maximum effect with minimum resources. Example: The Italian Game (1.e4 e5 2.Nf3 Nc6 3.Bc4) — every move develops toward the center and kingside, zero waste.
- MEDIUM (0.25-0.45): Mostly efficient with some minor tempo losses or redundancies. Example: The Chigorin variation (Ruy Lopez …Na5) — spends a tempo moving the knight to the rim, but with a concrete plan (Bxc4).
- LOW (0.05-0.20): Moves wasted (knight returns to starting square, rook shuffles aimlessly). Time management poor. Example: Moving the same piece three times in the opening (Nf3-g5-f3) — pure tempo waste.

## SOVEREIGNTY
Does this position preserve freedom of choice? Are future options kept open? Does the player maintain agency?
- HIGH (0.55-0.65): Flexible position with multiple plans. The player can adapt to the opponent's responses. Example: The English Opening (1.c4) — delays pawn structure commitment, keeps d4/e4/g3 options open.
- MEDIUM (0.25-0.45): Committed to a plan but retains some flexibility. Position has character without being rigid. Example: The Sicilian Najdorf (…a6) — committed to queenside counterplay but retains flexibility in piece placement.
- LOW (0.05-0.20): Overcommitted. Locked into one plan with no alternatives. Position plays itself (or is already lost). Example: A premature h4-h5-h6 pawn storm with no piece support — overcommitted, no way back.
