<!-- AI-CONTEXT
role: hackathon-constitution
audience: all team members + their AI coding assistants
purpose: single source of truth for hackathon rules, constraints, and compliance
priority: MUST be read before any code change during hackathon
cross-refs: [API.md, FRONTEND.md, CLAUDE.md]
-->

# Gemini 3 Paris Hackathon — Constitution

## Event

- **date:** 2026-03-14
- **hours:** 09:00–21:00 (submissions due 17:00)
- **location:** Wojo Coworking Paris 9e, 18 Rue de Londres
- **wifi_ssid:** GOOGLE
- **wifi_pass:** K3B56$on6k2
- **team_max:** 4
- **organizer:** Cerebral Valley
- **submission_url:** cerebralvalley.ai
- **submission_format:** 1-min video + public GitHub link + text description

---

## Rules — Verbatim Source

> "Model Usage: This hackathon may involve the use of closed-source models and APIs (limited to those provided by Google). Participants are permitted to use approved tools and models available for the hackathon, as long as they're accessible to all hackers. Teams are also allowed to build on top of existing open source projects. However, your demo must ONLY highlight the specific features, code, and functionality that your team built during the hackathon."

---

## Constraints (machine-parseable)

### ALLOWED

```yaml
closed_source_models:
  - provider: Google
    models: [gemini-2.5-flash, gemini-2.5-pro, gemini-2.0-flash]
    reason: "primary sponsor, credits provided"

open_source_models:
  - any model available via open-source tooling (llama.cpp, HF Transformers, etc.)
  - reason: "Teams are allowed to build on top of existing open source projects"

partners:
  - name: HuggingFace
    status: "official partner — resources announced day-of at kick-off"
    action: "wait for kick-off announcement before using HF Inference API"
  - name: Replit
    status: "official partner"

pre_existing_code:
  - CYNIC kernel (Rust backend) — open source, pre-hackathon
  - any open source library or framework
  - reason: "allowed to build on top of existing open source projects"
```

### FORBIDDEN

```yaml
forbidden:
  - rule: "NO closed-source non-Google models"
    examples: [Claude API, OpenAI API, Mistral API, Cohere API]
    consequence: "disqualification risk"

  - rule: "Demo must show ONLY hackathon work"
    meaning: "do not demo pre-existing kernel features as if built today"
    consequence: "disqualification risk"

  - rule: "Repo must be public"
    meaning: "GitHub repo must be publicly accessible at submission time"
```

### GRAY ZONE — decide on-site

```yaml
gray_zone:
  - item: "HuggingFace Inference API"
    status: "partner, but resources TBA at kick-off"
    decision: "ask organizers or wait for kick-off announcement"

  - item: "llama.cpp with open-source GGUF models running locally"
    status: "likely allowed (open source project rule)"
    decision: "verify with organizers if questioned"
```

---

## Judging

```yaml
criteria:
  - name: "Live Demo"
    weight: 0.45
    implication: "demo MUST work live — no slides, no video fallback"

  - name: "Creativity & Originality"
    weight: 0.35
    implication: "epistemic consensus under doubt = novel angle, lean into it"

  - name: "Impact Potential"
    weight: 0.20
    implication: "frame as universal tool, not niche"
```

---

## Pitch Format

### First Round (17:15–18:45)
- **duration:** ~3 min pitch + 1–2 min Q&A
- **outcome:** top 6 advance to final

### Final Round (19:00–20:00)
- **duration:** 3 min pitch + 2–3 min Q&A
- **audience:** everyone
- **winners announced:** 20:15

---

## Prizes

| Place | Prize |
|-------|-------|
| 1st | $50K Gemini credits + 1:1 session with Google AIFF founders |
| 2nd | $30K Gemini credits |
| 3rd | $20K Gemini credits |

---

## Compliance Checklist

<!-- AI: verify each item before submission at 17:00 -->

```
[ ] Gemini is the primary inference backend (Google = sponsor)
[ ] No Claude/OpenAI/Mistral/Cohere API calls in codebase
[ ] GitHub repo is public
[ ] Demo highlights ONLY code written during hackathon (2026-03-14)
[ ] 1-min video recorded and ready
[ ] Description submitted on cerebralvalley.ai
[ ] All team members registered
```

---

## Git Workflow

<!-- AI: enforce this workflow for every git operation during hackathon -->

```yaml
branch: main                # single branch — velocity over ceremony
sync: "git pull --rebase before every push"
commit_style: "type(scope): description"

ownership:
  frontend-dev:
    owns: [cynic-ui/**]
    never_touches: [cynic-kernel/**, scripts/**]
  backend-dev:
    owns: [cynic-kernel/**]
    never_touches: [cynic-ui/**]
  frozen:
    files: [API.md, HACKATHON-RULES.md, FRONTEND.md, CLAUDE.md]
    reason: "finalized pre-hackathon, do not modify during hackathon"

conflict_prevention:
  principle: "separate directories = zero merge conflicts"
  emergency: "if conflict occurs, the file OWNER resolves it"
```

---

## Video Submission

```yaml
what: 1-min screen recording of the working demo
deadline: 17:00 (same as code submission)
format: TBD — check at kick-off (likely YouTube link or direct upload)
who_records: S. (demo machine)
when_to_record: between 15:00 (feature freeze) and 16:45
tool: OBS Studio or phone camera pointed at screen
content:
  - show the judge page
  - submit a claim → show verdict animation
  - show axiom scores + dog comparison
  - show anomaly detection if possible
  - 60 seconds MAX
```

---

## Schedule with CYNIC Actions

| Time | Event | CYNIC Action |
|------|-------|-------------|
| 09:00 | Doors + breakfast | Connect WiFi, verify Tailscale, `git pull`, test `curl /health` |
| 10:00 | Kick-off | Listen for HF/Replit resource announcements, note what's approved |
| 12:00 | Lunch | Mid-check: frontend functional? API calls working? |
| 15:00 | — | **FEATURE FREEZE.** Polish UI, record video, write description |
| 17:00 | Submissions due | Submit video + GitHub + description |
| 17:15 | First round judging | 3 min pitch — focus on live demo (45% weight) |
| 19:00 | Final round | If top 6 — extended pitch |
| 20:15 | Winners announced | |

---

## CYNIC Hackathon Strategy

```yaml
dogs:
  - id: deterministic-dog
    role: "baseline — rule-based, instant, always available (no network needed)"
    status: deployed

  - id: gemini
    role: "primary AI evaluator — Google-provided, hackathon-compliant"
    model: gemini-2.5-flash
    status: deployed

  - id: huggingface
    role: "second AI evaluator — partner-provided, sovereignty signal"
    status: "activate after kick-off resource announcement"

  - id: llama-local
    role: "sovereignty dog — open source, runs on APU locally"
    status: "if time permits"

demo_machine: "S. Windows PC (desktop-mc9ffvt via Tailscale)"
kernel_machine: "T. Ubuntu (<TAILSCALE_UBUNTU> via Tailscale)"
demo_killer_feature: "multiple independent AI validators reaching consensus under mathematical doubt"
```
