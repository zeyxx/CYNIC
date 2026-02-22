# CYNIC Webapp Design Document

**Date**: 2026-02-21
**Status**: Design Phase - Ready for Implementation
**Author**: CYNIC
**Confidence**: 61.8% (φ⁻¹ limit)

## Executive Summary

Build a **type-driven web interface** enabling users to interact with CYNIC independently of Claude Code.

**Core Capabilities**:
- **B** (Invoke Commands): Execute operations via command palette
- **C** (Build & Extend): Create/edit Skills in browser
- **D** (Conversational): Chat foundation (future layer)

**Philosophy**: Types = source of truth. UI generated from type definitions. CYNIC self-introspects.

## Architecture

### Frontend
- TypeScript vanilla (no React/Vue)
- esbuild build (0.5s rebuild)
- Bundle: ~200KB minified
- WebSocket real-time updates

### Backend (New Endpoints)
```
POST /api/commands/invoke          (execute operation)
GET /api/commands/history          (command history)
GET /api/skills                    (list)
POST /api/skills                   (create)
PUT/GET/DELETE /api/skills/{id}    (CRUD)
GET /api/organism/schema           (type definitions ⭐)
GET /api/organism/state            (metrics)
WebSocket /ws                      (real-time events)
```

### Key Innovation: Schema Introspection
```
Backend exposes: GET /api/organism/schema (OrganismSchema)
├─ Commands + parameters as types
├─ Skills templates
├─ Organism state definition

Frontend uses schema to:
├─ Auto-generate command palette
├─ Build forms from parameter types
├─ Validate user input
└─ Display help text

CYNIC uses schema to:
├─ Discover own capabilities
├─ Validate skill creation
└─ Self-improve through type analysis
```

## Frontend Structure

```
webapp/
├─ public/index.html                    (single entry)
├─ src/
│  ├─ main.ts
│  ├─ types/api.ts                      (API types)
│  ├─ api/client.ts                     (REST + WS)
│  ├─ ui/command-panel.ts               (Flow B)
│  ├─ ui/skill-editor.ts                (Flow C)
│  ├─ ui/dashboard.ts
│  ├─ state/store.ts                    (simple state)
│  └─ util/                             (helpers)
├─ dist/                                (built output)
├─ package.json                         (typescript, esbuild only)
└─ esbuild.config.js
```

## User Flows

### Flow B: Invoke Command
1. Load webapp → fetch schema
2. Command palette rendered from schema
3. User searches/clicks command
4. Form auto-generated from param types
5. User fills form → clicks "Invoke"
6. Frontend validates against schema
7. POST /api/commands/invoke
8. Real-time response via WebSocket
9. Result displayed in dashboard

### Flow C: Create Skill
1. User clicks "New Skill"
2. Skill editor appears
3. User writes Python + clicks "Validate"
4. Backend: syntax check + security scan + type validation
5. Errors displayed inline OR skill loaded
6. Skill appears in command palette immediately

### Flow D: Chat (Prepared, Not MVP)
- Chat input field in UI (structure ready)
- Implemented in Phase 5

## Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| 1 | Week 1 | Framework + schema loading + Docker |
| 2 | Week 1+5 | Flow B: Commands fully working |
| 3 | Week 2 | Flow C: Skills fully working |
| 4 | Week 2+5 | Tests + polish + production ready |
| 5 | Week 3+ | Chat interface (future) |

## Success Criteria (MVP)

- [ ] User invokes ≥5 commands from UI
- [ ] User creates + validates skills
- [ ] 100% test coverage (critical paths)
- [ ] Bundle < 250KB
- [ ] 0 console errors
- [ ] Works: Chrome 90+, Firefox 88+, Safari 14+
- [ ] All tests passing

## Non-Functional

**Performance**: Bundle <300KB, First paint <1s, Command response <500ms
**Reliability**: Auto-reconnect WebSocket <5s, no data loss
**Security**: HTTPS, CORS same-origin, skill validation (no os.system)
**Accessibility**: Keyboard nav, screen reader support

## Deployment

```dockerfile
# Multi-stage Docker
FROM node:20-alpine AS builder
WORKDIR /app/webapp
COPY webapp .
RUN npm ci && npm run build

FROM python:3.11
COPY --from=builder /app/webapp/dist /app/cynic/static/
COPY cynic /app/cynic
RUN pip install -e .
EXPOSE 8000
CMD ["uvicorn", "cynic.api.entry:app", "--host", "0.0.0.0"]
```

## Decision Log

**TypeScript Vanilla (not React)**: Minimal deps, fast, scales elegantly
**Schema-Driven UI**: No hardcoding, CYNIC self-aware, extensible
**esbuild (not Webpack)**: Fast builds, simple config
**Chat deferred**: Foundation first, Phase 5

---

**Status**: Ready for Implementation Planning
**Next**: Invoke writing-plans skill to create detailed implementation plan
