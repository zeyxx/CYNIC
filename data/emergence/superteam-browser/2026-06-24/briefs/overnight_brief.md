# Superteam Browser Emergence Brief

Date: 2026-06-24

Root: `data/emergence/superteam-browser/2026-06-24/`

## What Was Built

I created a DATA-centric emergence base for Superteam Earn and the future browser/Superteam organs. The work stayed inside the dedicated repo zone and did not submit, sign, post, trade, stake, deposit, or mutate any account.

The current inventory contains:

- 33 global open Earn bounties.
- 33 normalized bounty detail records.
- 44 live grants.
- 38 Discord Superteam France leads.
- 8 browser/platform observations.

## Platform Shape

Superteam Earn currently behaves like a layered opportunity system, not a single flat list:

- Global bounties: strongest for immediate scoring and safe preparation.
- Grants: durable funding, slower, higher human strategy load.
- Discord France leads: local/community edge, but must be freshness-checked.
- Authenticated areas: bookmarks, dashboard, profile, and agent surfaces.
- Agent listings: probed endpoint returned Unauthorized, so this is a separate gated layer.

The UI has enough filters, sorts, tabs, detail pages, and auth-only surfaces to justify a real `organ-browser`, not just ad hoc scraping.

## Emerging Organs

Formalized specs:

- `organ-browser`: search, extract, interact, evidence capture, mutation gate.
- `organ-superteam`: opportunity inventory, scoring, submission pipeline, outcome learning.
- `organ-evidence`: claim/source/artifact bundles.
- `organ-portfolio`: reusable credibility blocks for grants and subjective bounties.
- `organ-research`: structured search-extract-interact research.
- `organ-submission`: reviewer-ready drafts with hard human gates.

The key architectural rule is clear: browser access must become a policy-wrapped organ. Raw CDP is useful but too sharp unless every action is classified and logged.

## Emerging Agent Skills

Formalized skills include:

- `web_route_mapper`
- `filter_pagination_walker`
- `listing_requirement_extractor`
- `bounty_classifier`
- `safe_submission_form_prober`
- `evidence_bundle_creator`
- `human_gate_requester`
- `agent_skill_packager`
- `bug_bounty_finding_writer`
- `grant_proposal_skeletonizer`
- `retrospective_learner`

The most immediate reusable skill is `agent_skill_packager`, because it maps directly onto the Solana AI Kit bounty and also improves the organism.

## Opportunity Layers

The practical operating layers are:

- Quick cash / low-risk: low mutation, low public identity exposure, easy evidence.
- Technical reusable: best default target; work compounds into CYNIC even if no payout.
- Grants / durable funding: high strategic value, human-led.
- Social/content subjective: potentially lucrative but needs human voice and public posting approval.
- Discord/community/stale leads: useful edge layer, must be reconciled against Earn.

The v1 scorer currently needs a bias fix: high-reward content bounties with AI/Solana keywords can rank too high. Tomorrow, the scorer should explicitly separate "technical reusable" from "technical words in a content task."

## Started Work

1. `started_work/skills/`

Prepared a candidate submission for the Solana AI Kit skill bounty:

`solana-bounty-to-submission-skill`

This is a public-safe skill concept that helps agents turn bounty/grant pages into requirement maps, evidence bundles, submission plans, and human-gated payloads.

Status: draft skill and submission plan prepared, not submitted.

2. `started_work/break-it-before-users-do-multihopper-agentic-flow-bugs-and-fixes/`

Prepared a QA matrix and finding template for the MultiHopper bounty. The actual tests require sponsor-approved API keys, wallet, balances, and test scope.

Status: test matrix prepared, no transfer executed.

3. `started_work/advanced-infrastructure-challenge-build-a-smart-transaction-stack/`

Prepared an architecture outline. This bounty is high learning value but needs real infra, Geyser/Yellowstone, Jito, wallet policy, and budget.

Status: architecture prepared, no bundle submission executed.

## Recommended Morning Decisions

1. Approve or reject turning the Solana bounty-to-submission skill draft into a public repo or PR.
2. Decide whether to request MultiHopper test credentials and run the QA matrix.
3. Decide whether the Smart Transaction Stack is worth a focused sprint before its 2026-06-29 deadline.
4. Fix kernel/MCP reachability so future overnight work can persist observations in the kernel, not only in the repo zone.
5. Add a scorer v2 that treats social posting, public identity, and content subjectivity as separate penalties.

## Safety Notes

No cookies, tokens, private keys, personal Google data, authenticated raw HTML dumps, or secrets were written.

No external mutation was performed.

The kernel coordination path was unavailable through the visible MCP endpoint, so the blocker was recorded at `observations/coord_blockers.jsonl`.
