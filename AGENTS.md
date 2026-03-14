# CYNIC — Multi-Agent Coordination

Two developers, two Claude Code instances, one repo, one branch.

## Agents

| Agent | Human | Zone | Device |
|---|---|---|---|
| Backend Agent | T. | `cynic-kernel/`, root docs, scripts/ | Ubuntu desktop (<TAILSCALE_UBUNTU>) |
| Frontend Agent | S. | `cynic-ui/` only | Windows desktop (<TAILSCALE_STANISLAZ>) + Replit |

## Conflict Prevention

Separate directories = zero merge conflicts. This ONLY works if:
1. **Never touch files outside your zone.** Backend agent never edits cynic-ui/. Frontend agent never edits cynic-kernel/.
2. **`git pull --rebase` before every push.** Always.
3. **Root files are frozen:** CLAUDE.md, API.md, FRONTEND.md, AGENTS.md, HACKATHON-RULES.md — do not modify during hackathon.

## Communication Protocol

Changes to the API contract (API.md) go through T. only. If the frontend needs a new endpoint or field:
1. Frontend agent documents what's needed in a comment or message
2. Backend agent implements it, updates API.md, pushes
3. Frontend agent pulls and adapts

## Deploy Workflow

### Backend (T.)
```
Code change → /deploy (build+test+clippy+restart) → /test-chess → git commit + push
```

### Frontend (S.)
```
Code change → npm run dev (local preview) → git commit + push → Replit auto-pulls or manual deploy
```

## Infrastructure

The CYNIC kernel runs on T.'s Ubuntu machine and is the SINGLE SOURCE OF TRUTH for inference.
- Tailscale: http://<TAILSCALE_UBUNTU>:3030 (direct, requires Tailscale)
- Public tunnel: https://associations-mailed-treasury-component.trycloudflare.com (Cloudflare, no auth needed)

Frontend (cynic-ui) connects to the kernel API. It NEVER does inference directly.

## Hackathon Timeline

- 12h-16h: Code (backend chess logic + frontend UI)
- 16h-16h30: Integration test, fix bugs
- 16h30-17h: Record 1-min demo video, submit
- 17h: Submission deadline
- 17h15: Judging (3 min pitch + Q&A)
