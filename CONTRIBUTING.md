# Contributing to CYNIC

Welcome! 🙌 CYNIC is a self-observing AI judgment oracle for memecoin governance. We love contributions from people at all levels.

## For New Contributors (Start Here!)

**You don't need a laptop to start contributing.** We have issues designed for:
- 📱 **Mobile-first** — Read code, understand architecture, write documentation
- 💻 **When you get hardware** — Code implementation, testing, debugging

## Getting Started (Mobile-Friendly Path)

### Week 1: Learn the Architecture
1. **Read** the CYNIC overview in `docs/CYNIC_COMPLETE_SYSTEM_MAP.md`
2. **Understand** the 11 Dogs + Gossip Federation (Session 5)
3. **Pick a "Good First Issue"** from GitHub Issues

### Week 2: Document Your Learning
1. **Write** a short explanation (200-500 words) of what you learned
2. **Submit** as a PR to `docs/contributor-learning/`
3. **Get feedback** from maintainers

### Week 3+: Code (When You Have Hardware)
1. **Pick a coding issue** from GitHub Issues
2. **Set up environment** (see below)
3. **Submit a PR** with tests

---

## How to Contribute

### Option A: Documentation (Mobile ✅)
**Perfect for:** Learning the codebase without coding
**Examples:**
- Explain how Gossip Protocol works
- Document the 11 Dogs decision-making
- Create architecture diagrams (text or simple)
- Write beginner-friendly guides

**How to submit:**
1. Create file in `docs/contributor-learning/your-name-topic.md`
2. Write your explanation (clear, honest, no jargon)
3. Open PR with title: `docs: Add contributor learning - [topic]`

### Option B: Testing (Mobile Partial ⚠️)
**Perfect for:** Understanding test structure
**Examples:**
- Write test descriptions (what each test checks)
- Explain test failure scenarios
- Document edge cases

**How to submit:**
1. Update `cynic/tests/test_*.py` docstrings
2. Add clear test comments
3. Open PR with title: `test: Improve test documentation for [module]`

### Option C: Code Implementation (Laptop 💻)
**Perfect for:** When you have development hardware
**Requirements:**
- Python 3.13+
- Git + GitHub
- Running tests locally

**Process:**
1. Fork the repo
2. Create branch: `git checkout -b feat/your-feature`
3. Make changes with tests
4. Run: `pytest cynic/tests/`
5. Push and open PR

---

## Setup for Development

### Requirements
- Python 3.13+
- `pip` (included with Python)
- Git

### Installation
```bash
# Clone the repo
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC

# Create virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install dependencies
pip install -e .

# Run tests
pytest cynic/tests/ -v
```

### Before You Code
1. Read `docs/CYNIC_COMPLETE_SYSTEM_MAP.md`
2. Understand the architecture (Sovereignty → Emergence → Coordination)
3. Run existing tests to verify setup: `pytest cynic/tests/test_federation.py -v`

---

## Code Style & Standards

**We follow these principles:**

1. **Immutability First**
   ```python
   from dataclasses import dataclass

   @dataclass(frozen=True)
   class MyData:
       value: str
   ```

2. **Type Hints Always**
   ```python
   def process(data: str) -> int:
       return len(data)
   ```

3. **Tests First (TDD)**
   - Write test before code
   - All tests must pass: `pytest cynic/tests/ -x`
   - Target 99%+ pass rate

4. **φ-Bounded Values**
   - Max confidence = 0.618 (φ⁻¹, golden ratio)
   - All probabilities clamped to [0, 1]

5. **No Stubs**
   - Every function must work
   - No `pass` or `NotImplemented` in production code
   - Use `pytest.skip()` for incomplete tests

---

## PR Process

1. **Open Issue First** (unless it's a tiny fix)
   - Describe what you're doing
   - Ask questions if unclear
   - Get approval before coding

2. **Branch Naming**
   - Feature: `feat/your-feature-name`
   - Fix: `fix/issue-description`
   - Docs: `docs/topic-name`

3. **Commit Messages**
   ```
   feat: Add GossipManager P2P protocol

   - Implements k=3 peer gossip
   - Weighted Q-Table merge with φ-bounds
   - 4 unit tests + 1 integration test

   Co-Authored-By: Your Name <your-email>
   ```

4. **PR Description**
   - Link to related issue: `Closes #123`
   - Explain what changed and why
   - Show test results: `pytest output`
   - Include before/after if visual

5. **Merge Requirements**
   - ✅ All tests passing
   - ✅ Code review approved
   - ✅ No merge conflicts
   - ✅ Documentation updated

---

## "Good First Issue" Guide

These issues are labeled **`good-first-issue`** and designed for new contributors.

### Example: Document the Gossip Protocol
**Task:** Write a 300-word explanation of how CYNIC's P2P Gossip Federation works

**What to read:**
- `cynic/federation/gossip.py` (the code)
- `docs/PHASE3_UNIFIED_ARCHITECTURE.md` (design context)
- Session 5 commit messages

**What to write:**
- How many peers does each instance connect to? (k=3)
- What gets shared? (Q-Table snapshots)
- When does it trigger? (every 10 judgments)
- Why is it useful? (scalable learning)

**Where to put it:**
- `docs/contributor-learning/your-name-gossip-explained.md`

**How to submit:**
```bash
git checkout -b docs/gossip-explanation
# (write your file)
git add docs/contributor-learning/your-name-gossip-explained.md
git commit -m "docs: Add contributor learning - Gossip Protocol explained"
git push origin docs/gossip-explanation
# (open PR on GitHub)
```

---

## Questions?

- **Architecture unclear?** Comment on the issue or open a Discussion
- **Setup problems?** Ask in Discord or GitHub Issues
- **Not sure what to work on?** Read `Good First Issues` and pick one

---

## Code of Conduct

- Be respectful and inclusive
- Assume good intent
- Give credit generously
- Help others learn
- No spam or self-promotion

---

## Recognition

Contributors are:
- Added to CONTRIBUTORS.md
- Mentioned in release notes
- Eligible for compensation if CYNIC generates revenue

Thank you for helping build CYNIC! 🚀
