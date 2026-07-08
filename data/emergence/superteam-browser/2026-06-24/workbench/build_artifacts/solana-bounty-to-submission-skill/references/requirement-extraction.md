# Requirement Extraction

Extract opportunity requirements into JSON with these fields:

- `source.url`
- `source.observed_at`
- `source.status`
- `opportunity.title`
- `opportunity.sponsor`
- `opportunity.deadline`
- `opportunity.reward`
- `deliverables[]`
- `judging_criteria[]`
- `submission_format`
- `resources[]`
- `disqualifiers[]`
- `hidden_gates[]`
- `open_questions[]`

Hidden gates include:

- public social posting
- identity or location constraints
- wallet connection, signature, token transfer, deposit, stake, trade, or funds
- sponsor-provided API keys, test credentials, credits, or balances
- private repo, Google account, Discord, Telegram, or authenticated dashboard
- security disclosure restrictions

If a field is absent, mark it `unknown`; do not infer missing requirements as
safe.
