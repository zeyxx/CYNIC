# Agency & Privacy Doctrine

Agency and privacy are governing constraints, not preferences. Every fleet
mechanism is judged against both.

## Agency

- The human delegates; agents execute within typed, leased, audited bounds.
- An agent that cannot complete without exceeding its bounds reports
  `blocked` with the exact missing capability. Improvisation is a defect.
- No agent acts on ambiguity: silence is not consent, an expired lease is
  not consent, a model prior is not authorization.
- Delegation is monotonic and revocable: leases expire, scopes narrow
  downward, the human can always see and always stop.

## Privacy

- Data minimizes across every boundary: the DMZ holds no CYNIC secrets,
  agents never carry keys they do not need, receipts carry proofs not contents.
- Peer projects (GASdf and others) keep their keys and pipelines; hosting
  never becomes custody.
- Public exposure is explicit and enumerated (firewall intent, DNS both
  sides); anything reachable but undeclared is an incident, not drift.
- The human's data (keys, history, vault) never enters agent prompts,
  reports, or telemetry — presence and provenance only.

## Test

Any proposal that widens agency without a matching audit trail, or widens
exposure without a matching boundary proof, is refused at design review.
