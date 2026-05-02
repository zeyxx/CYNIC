# External Projects — Absorption Roadmap

This directory contains external projects cohabiting in the CYNIC workspace. They are untracked (in `.gitignore`) and represent either:
1. **T.'s projects** planned for absorption into CYNIC core modules
2. **Collaborative projects** (ASDFASDFA's verification system) with uncertain integration path
3. **Forks** that will be refactored/merged

## Projects

### T.'s Original Projects (Planned Absorption)

#### HolDex
- **Location:** `external/HolDex/`
- **Purpose:** Holder behavior indexing, anti-Sybil scoring
- **Status:** Planned absorption into `cynic-indexing/` (core module)
- **Integration:** Wallet behavior analysis pipeline
- **Type:** Original

#### ASDev
- **Location:** `external/ASDev/`
- **Purpose:** App development platform
- **Status:** Planned absorption into `cynic-apps/` or `cynic-node/`
- **Integration:** TBD (T. to clarify)
- **Type:** Original

#### asdf_grinder
- **Location:** `external/asdf_grinder/`
- **Purpose:** TBD (T. to clarify)
- **Status:** Planned absorption into core
- **Integration:** TBD
- **Type:** Original

#### ASDFBurnTracker
- **Location:** `external/ASDFBurnTracker/`
- **Purpose:** BURN/efficiency metric tracking
- **Status:** Planned absorption into observability pipeline
- **Integration:** TBD (T. to clarify)
- **Type:** Original

### Collaborative Projects

#### CultScreener
- **Location:** `external/CultScreener/`
- **Owner:** ASDFASDFA
- **Purpose:** Human verification + wallet verification
- **Status:** UNCERTAIN (active in hackathon, post-hackathon absorption unclear)
- **Integration:** Optional integration via `/mint-permit` flow (B&C + CYNIC Dogs)
- **Contact:** ASDFASDFA (S.)

---

## Absorption Timeline

**Phase 1 (Near future):**
- HolDex → `cynic-indexing/` (wallet behavior, anti-Sybil)
- ASDev → TBD (clarify first)
- ASDFBurnTracker → TBD (clarify first)

**Phase 2 (Post-hackathon):**
- asdf_grinder → clarify + absorb
- CultScreener → TBD (depends on S.'s decision)

---

## LLM Context Access

When searching for knowledge on these domains, check `external/`:
- **Holder behavior:** `external/HolDex/`
- **Human verification:** `external/CultScreener/`
- **Wallet authenticity:** `external/HolDex/` + `external/CultScreener/`
- **App development:** `external/ASDev/`
- **Observability:** `external/ASDFBurnTracker/`

---

## Git Handling

All external/ projects are gitignored (not tracked in CYNIC repo). Each maintains own version history:
```bash
# View history of a specific project:
cd external/HolDex && git log
```

When absorbing into core:
1. Copy code from `external/<project>` into `cynic-<module>/`
2. Merge commit history or reset to specific commit
3. Delete `external/<project>/` directory
4. Update EXTERNAL_CONTEXT.md

---

## Fork Status

**To be clarified by T.:**
- Which projects are forks vs. originals?
- Which have upstream repos?
- Which need upstream sync before absorption?

Update this section once clarified.
