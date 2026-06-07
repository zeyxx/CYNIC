# PoH Sync for S. Codex — 2026-06-07

Purpose: make the Talaria x Blitz & Chill PoH integration context immediately available before Ownership Radio.

## Read First

1. `docs/architecture/talaria-observatory.md`
   - Canonical CYNIC architecture.
   - Defines `talaria.poh.user`, `talaria.chess.signal`, public safety rules, and the MVP sequence.

2. `docs/architecture/talaria-metabolic-soc.md`
   - Operational separation of concerns.
   - Defines B&C as PoH producer, CYNIC as observatory, Telegram as public consumer/enforcement.

3. `cynic-python/sensors/talaria_poh_bridge.py`
   - B&C API -> CYNIC `/observe` sensor.

4. `cynic-python/organs/telegram/bot.py`
   - Public Telegram `/verify <wallet>` consumer.

## Current Minimal PoH Path

```text
user joins public Telegram
  -> bot restricts user
  -> user plays one B&C chess game
  -> user runs /verify <wallet>
  -> Telegram bot calls B&C /api/verify
  -> bot writes CYNIC observation domain=talaria.poh.user
  -> bot unmutes user if verified=true
```

This is the launch path, not the final strongest identity model. Later, replace `/verify <wallet>` with a server-authenticated B&C callback binding `telegram:<id>` to `wallet:<addr>` through CYNIC.

## API Contract Expected from B&C

```text
GET https://blitzchill.space/api/verify?wallet=<wallet>
```

Expected JSON:

```json
{
  "verified": true,
  "wallet": "<wallet>",
  "archetype": "optional chess/personality archetype",
  "assetAddress": "optional soulbound asset",
  "confidence": 0.8,
  "issuedAt": "2026-06-07T00:00:00Z"
}
```

If not verified, return `verified: false` and preferably a short `reason`.

## Runtime Env

Telegram bot:

```text
CYNIC_TELEGRAM_BOT_TOKEN=<bot token>
TALARIA_PUBLIC_GROUP_ID=<telegram group id>
TALARIA_OPS_CHAT_ID=<private ops chat id>
BC_API_URL=https://blitzchill.space/api/verify
CYNIC_REST_ADDR=<kernel url>
CYNIC_API_KEY=<kernel bearer token>
```

PoH bridge:

```text
BC_API_URL=https://blitzchill.space/api/verify
CYNIC_REST_ADDR=<kernel url>
CYNIC_API_KEY=<kernel bearer token>
```

Do not commit real tokens, chat IDs, private Tailscale IPs, or API keys.

## Ownership Boundaries

- B&C owns chess play, Privy/session continuity, wallet binding, PoH evidence, and public PoH API.
- CYNIC owns Talaria event semantics, `/observe`, judgment, observatory state, reputation, and publication filtering.
- Telegram owns public interaction and group enforcement only.
- X/Ops remains separate; Telegram public PoH must not trigger X posting or Ops decisions as hidden side effects.

## Immediate Setup Checklist

- [ ] Deploy or verify B&C branch with `/api/verify`.
- [ ] Probe `https://blitzchill.space/api/verify?wallet=<known_verified_wallet>`.
- [ ] Run Telegram bot with `BC_API_URL=https://blitzchill.space/api/verify`.
- [ ] Ensure bot is admin in the public group and can restrict/unrestrict members.
- [ ] Test `/verify <wallet>` from the public group.
- [ ] Confirm a CYNIC observation appears with `domain=talaria.poh.user`.
- [ ] Keep claims modest on radio: one game grants launch/community access; full governance trust remains review/calibration.

## Stable B&C Baseline Mentioned in Handoff

Use this branch if S. needs the latest reconciled B&C baseline:

```text
https://github.com/zeyxx/blitz-and-chill/tree/feat/reconcile-privy-ux
```

Handoff note says it fixes the SIGSEGV, restores design, and has functional Privy middleware for socket-server.

## Radio Framing

Short version:

```text
CYNIC judges proposals. Blitz & Chill proves participants are human through chess.
No KYC, no biometrics, no cloud dependency for judgment. Off-chain compute, on-chain settlement later.
For launch: play one game, verify a wallet, unlock Talaria community access.
```
