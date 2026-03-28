# CYNIC — Multi-Agent Coordination

Two developers, two Claude Code instances, one repo, one branch.

## Agents

| Agent | Human | Zone | Device |
|---|---|---|---|
| Backend Agent | T. | `cynic-kernel/`, root docs, scripts/ | Ubuntu desktop (<TAILSCALE_CORE>) |
| Frontend Agent | S. | `cynic-ui/` only | Windows desktop (<TAILSCALE_GPU>) + Replit |

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
- API: `http://<TAILSCALE_CORE>:3030` (Tailscale only, Bearer auth required)
- No public tunnel. No Cloudflare. No ngrok. Ever.

Frontend (cynic-ui) connects to the kernel API. It NEVER does inference directly.

## Security Rules (both agents)

**This repo is PUBLIC.** Every commit is visible to the world.

1. **Never commit secrets.** API keys, tokens, passwords, real IPs, real names → env files only.
2. **Use placeholders** for infrastructure: `<TAILSCALE_CORE>`, `<TAILSCALE_GPU>`, etc.
3. **Use initials** for people: `T.`, `S.` — never full names.
4. **Verify before pushing:** `git diff --staged | grep -iE "api.key|token|password|AIza|hf_|100\.(74|75|119)"` must return empty.
5. **Never open a public tunnel** (Cloudflare, ngrok) without auth on the API.
6. **All API calls require Bearer auth** — `Authorization: Bearer $CYNIC_API_KEY`.
