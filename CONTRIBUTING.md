# Contributing to CYNIC

> *"φ distrusts φ"* - We welcome skeptical contributions.

---

## Quick Start

1. Fork the repository
2. Clone your fork
3. Install dependencies: `npm install`
4. Run tests: `npm test`
5. Create a branch for your changes
6. Submit a pull request

---

## Development Guidelines

### Code Style

- Pure JavaScript (ES modules)
- Named exports only
- φ-aligned constants where applicable
- Structured logging via `@cynic/core/logger`

### Testing

All code must be tested. Run the test suite:

```bash
npm test                    # All tests
node --test packages/*/test/*.test.js  # Specific packages
```

### Commit Messages

Follow conventional commits:
- `feat(package): description`
- `fix(package): description`
- `test(package): description`
- `docs: description`

---

## Priority Areas

1. **Test coverage** - See task list for gaps
2. **Documentation** - Keep docs in sync with code
3. **Solana integration** - Burns, E-Score anchoring
4. **ZK circuits** - Noir proof generation

---

## Quarterly Documentation Review

Every quarter (January, April, July, October), maintainers should perform a documentation audit:

### 1. ROADMAP.md Review
- [ ] Verify all dates are current (not past-due without updates)
- [ ] Check completion status matches actual implementation
- [ ] Update metrics and progress percentages
- [ ] Archive completed milestones if needed

### 2. ARCHITECTURE_LIVE.md Verification
- [ ] Trace code paths mentioned against actual codebase
- [ ] Verify file paths still exist and are accurate
- [ ] Check for new components not yet documented
- [ ] Update "Generated" date after verification

### 3. Audit Files Review (`docs/audits/`)
- [ ] Check if audit findings have been addressed
- [ ] Archive consumed/resolved audits (or add "RESOLVED" tag)
- [ ] Ensure no stale audits remain without action items

### 4. "Last Updated" Dates
Update dates in all major docs:
- [ ] `docs/ARCHITECTURE.md`
- [ ] `docs/DOGS.md`
- [ ] `ROADMAP.md`
- [ ] All `packages/*/README.md` files

### 5. Link Verification
- [ ] Run link checker on all markdown files
- [ ] Fix broken internal links
- [ ] Verify external links still work

### Review Checklist Template

```markdown
## Q[N] 20XX Documentation Review

**Reviewer**: [name]
**Date**: YYYY-MM-DD

- [ ] ROADMAP.md dates verified
- [ ] ARCHITECTURE_LIVE.md traced
- [ ] Audit files reviewed
- [ ] "Last Updated" dates refreshed
- [ ] Links verified

**Notes**:
- [any issues found]
```

---

## Architecture

See [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) for technical design.

Key principles:
- **PHI** (φ): Golden ratio in timing, thresholds, confidence
- **VERIFY**: Don't trust, verify everything
- **CULTURE**: Community is the moat
- **BURN**: 100% burn economy, no extraction

---

## Questions?

Open an issue or reach out via the repository.

---

*κυνικός | Loyal to truth, not to comfort*
