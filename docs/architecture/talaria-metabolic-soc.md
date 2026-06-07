# Talaria Metabolic SoC

Status: active design note. Protocol still lives in `AGENTS.md`; this document defines operational separation of concerns for Talaria PoH, Telegram, Ops, and X.

## Core Rule

Telegram Public, Telegram Ops, X, B&C, and CYNIC are separate metabolic surfaces. They exchange observations, tasks, and verdicts through CYNIC. They do not call each other as hidden side effects.

## Surfaces

### B&C PoH Product

Owns chess play, Privy/session identity, wallet binding, personality cards, and the public PoH API.

Produces:

```text
GET /api/verify?wallet=<wallet>
-> { verified, wallet, archetype, assetAddress, confidence, issuedAt }
```

Does not own Telegram permissions, Telegram copy, X posting, or Talaria Ops decisions.

### CYNIC Kernel

Owns the metabolic bus: `/observe`, `/judge`, `/agent-tasks`, verdicts, claims, and durable observations.

Consumes B&C PoH signals as Talaria observations. Consumes X engagement opportunities as Ops validation tasks. CYNIC is not a UI and should not be bypassed for inter-organ decisions.

### Talaria PoH Bridge

`cynic-python/sensors/talaria_poh_bridge.py` is a sensor only.

Flow:

```text
B&C /api/verify -> talaria.event.v1 -> CYNIC /observe domain=talaria.poh.user
```

It does not unmute Telegram users and does not judge humanity by itself.

### Telegram Public

`@TalariaBuild_bot` public handlers own community interaction and public group access.

Allowed responsibilities:

- `/start`, `/help`, `/judge`, `/ask`, `/observe`
- `/verify <wallet>` as the public PoH consumer
- new-member restriction in `TALARIA_PUBLIC_GROUP_ID`
- unrestricting a user after B&C reports the wallet as verified
- writing a CYNIC observation for each verification attempt

Forbidden responsibilities:

- X posting
- Ops approval buttons
- private incident workflow
- deciding final truth without a B&C/CYNIC signal

### Telegram Ops

Telegram Ops is private human validation. It is addressed by `TALARIA_OPS_CHAT_ID`.

Allowed responsibilities:

- receive `engagement-validation` tasks
- approve or reject proposed X actions
- receive operational alerts

Forbidden responsibilities:

- public onboarding
- public PoH enforcement
- community chat ingestion as if it were public signal

### X Organ

The X organ owns perception, analysis, drafting, and posting for `@TalariaBuild`.

Flow:

```text
X perception -> analysis -> draft -> CYNIC agent-task kind=engagement-validation -> Telegram Ops -> X poster
```

It does not read Telegram public PoH state and does not post without Ops approval.

### Personal Telegram Account / Listener

`telegram-listener.service` uses the Telegram user API/MTProto. It is an ingestion organ, not `@TalariaBuild_bot`.

Owns:

- channel listening
- message buffering
- local SQLite ingestion
- signal classification via `organs.telegram.pipeline`

Does not own:

- bot commands
- public group permissions
- Ops approval buttons

## Current Runtime Map

| Surface | Runtime | Main file | Environment | Role |
|---|---|---|---|---|
| Telegram bot | `python3 cynic-python/organs/telegram/bot.py` | `cynic-python/organs/telegram/bot.py` | `CYNIC_TELEGRAM_BOT_TOKEN`, `TALARIA_PUBLIC_GROUP_ID`, `TALARIA_OPS_CHAT_ID`, `BC_API_URL` | Public commands + Ops buttons (Service: `telegram-bot.service`) |
| Telegram listener | `telegram-listener.service` | `cynic-python/organs/telegram/listener.py` | `TELEGRAM_API_ID`, `TELEGRAM_API_HASH` | Personal-account ingestion |
| B&C PoH API | Next.js | `apps/web/src/app/api/verify/route.ts` in B&C repo | B&C app env/db | PoH producer |
| PoH bridge | script/sensor | `cynic-python/sensors/talaria_poh_bridge.py` | `CYNIC_REST_ADDR`, `CYNIC_API_KEY`, `BC_API_URL` | B&C -> CYNIC observation |
| X organ | Hermes X | `cynic-python/organs/hermes_x/*`, `scripts/hermes-x/core/*` | X/browser/CYNIC env | X perception + Ops-gated action |

### Role of Intelligence (Dogs & Cortex)

The Telegram surface consumes intelligence through two primary channels:

1. **Dogs (In-Kernel Validators)**:
   - Used via the `/judge` command in the bot.
   - Includes sovereign models (local Qwen) and non-sovereign models (Gemini-CLI, HF).
   - Provides deterministic or probabilistic scores on the 6 CYNIC axioms.

2. **Gemini (Asymmetric Reasoning)**:
   - **As a Cortex CLI**: Used by T. for interactive reasoning sessions, session synthesis, and architectural decisions.
   - **As an Inference Dog**: Acting as a non-sovereign validator for content, providing cross-family model diversity (architectural independence).

## Integration Rules

1. A public Telegram handler may consume B&C/CYNIC state, but must not create Ops tasks.
2. An Ops handler may complete CYNIC tasks, but must not modify public group membership except through a separately named admin workflow.
3. X may create `engagement-validation` tasks, but only Ops may approve them.
4. B&C may publish PoH state, but must not mutate CYNIC or Telegram without an explicit bridge/callback contract.
5. CYNIC observations are the metabolic memory; direct hidden coupling is a bug.

## Current Minimal PoH Path

The immediate unblock path is intentionally simple:

```text
user joins public Telegram -> bot restricts user -> user plays B&C -> user runs /verify <wallet> -> bot calls B&C /api/verify -> bot observes result in CYNIC -> bot unmutes user when verified=true
```

This is not the final strongest identity model. The stronger later path is a B&C callback that binds `telegram:<id>` to `wallet:<address>` through CYNIC with server-authenticated evidence.
