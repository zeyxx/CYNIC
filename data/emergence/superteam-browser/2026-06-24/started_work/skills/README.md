# Started Work: Solana AI Kit Skill Bounty

Listing: https://superteam.fun/earn/listing/skills/

Decision: prepare a public-safe skill package concept, but do not submit or open a PR without human approval.

## Candidate Submission

Name: `solana-bounty-to-submission-skill`

Purpose: help Solana builders and agent teams turn bounty/grant pages into requirement maps, evidence bundles, reviewer-ready drafts, and human-gated submissions.

Why it fits:

- The Solana AI Kit values practical agent skills, progressive loading, subagents, workflow commands, and MCP-aware builder flows.
- Superteam Earn is real terrain with messy listings, deadlines, reward structures, public docs, Discord leads, and submission gates.
- The skill is reusable even if the bounty is not won: it becomes an organism capability for future grants, audits, hackathons, and ecosystem work.

## Proposed Package

- `skill/SKILL.md`: entry point and trigger rules.
- `skill/references/requirement-extraction.md`: how to extract deliverables, judging criteria, deadlines, and hidden gates.
- `skill/references/evidence-bundles.md`: how to map claims to sources, hashes, screenshots, and sanitized logs.
- `skill/references/human-gates.md`: exact rules for submit/sign/post/wallet/capital approval.
- `examples/superteam-earn.json`: redacted example from the current dataset.
- `tests/fixtures/listing.md`: small synthetic listing to test extraction.
- `tests/expected/requirements.json`: expected extraction output.

## Human Review Questions

- Should this be submitted as a standalone repo or as a PR to `solanabr/skill-bounty`?
- Should the public framing mention CYNIC, or should it stay generic for Solana builders?
- Do we want the skill to include browser automation, or keep it limited to extraction and submission preparation?
