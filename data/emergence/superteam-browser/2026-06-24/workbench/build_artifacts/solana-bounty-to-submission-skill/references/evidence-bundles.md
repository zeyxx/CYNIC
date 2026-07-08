# Evidence Bundles

An evidence bundle proves what was actually observed or built.

Use this structure:

```json
{
  "claims": [
    {
      "claim": "The submission includes a reproducible QA matrix.",
      "kind": "artifact",
      "evidence": "test_matrix.json",
      "status": "observed"
    }
  ],
  "artifacts": [],
  "sanitized_logs": [],
  "screenshots": [],
  "external_references": [],
  "inferences": [],
  "redactions": []
}
```

Rules:

- Label untested ideas as `planned` or `inferred`, never `verified`.
- Redact secrets, cookies, tokens, wallet addresses if private, account emails,
  and personal Google data.
- For security or QA bounties, include expected result, actual result, impact,
  reproduction path, and proposed fix.
- For transaction or on-chain claims, include network, slot, signature, explorer
  URL, timestamp, and what was controlled by the agent.
