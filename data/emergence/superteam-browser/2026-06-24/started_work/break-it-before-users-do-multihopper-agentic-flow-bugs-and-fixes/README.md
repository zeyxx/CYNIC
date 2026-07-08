# Started Work: MultiHopper Agentic QA Bounty

Listing: https://superteam.fun/earn/listing/break-it-before-users-do-multihopper-agentic-flow-bugs-and-fixes/

Decision: prepare the QA matrix and report format. Do not execute transfers until MultiHopper-approved test credentials, wallets, balances, and scope are explicitly available.

## Scope Extract

The bounty wants agentic workflow issues around:

- API-managed create, prepare, confirm-broadcast, and monitor lifecycle.
- Client-managed signing and broadcasting.
- Strict broadcast order.
- Two-step `confirm-broadcast`.
- Idempotency, retries, expiry, resume.
- Status polling and webhook monitoring.
- Documentation blockers that materially affect agent integrations.

## Main Test Hypotheses

- Agents may overwrite partial server signatures when signing VersionedTransactions.
- Agents may broadcast route/session transactions before keeper funding is confirmed.
- Agents may call `confirm-broadcast` only once, raising double-funding or resume ambiguity.
- Idempotency keys may be reused, omitted, or regenerated at the wrong boundary.
- Expired blockhash recovery may incorrectly re-broadcast already confirmed groups.
- Documentation examples may be insufficiently explicit for safe autonomous execution.

## Required Human/Sponsor Gates

- Sponsor-approved test API key.
- Sponsor-approved wallet and token balances.
- Confirmation that devnet/test environment is in scope.
- No mainnet or live user funds without written approval.
