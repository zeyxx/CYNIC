# Contributing to CYNIC

Thank you for your interest in contributing to CYNIC. 

CYNIC is a low-layer sovereign Rust runtime built on rigorous architectural boundaries, deterministic verification, and strict proof invariants. All contributions must adhere to these standards.

---

## Core Invariants

Before writing code, ensure your design complies with our constitutional rules:

1. **Strict DAG Downward**: Higher abstraction layers depend on lower layers, never the reverse. Pure domain crates (`cynic-protocol`, `cynic-types`, etc.) must **never** depend on orchestrators or host crates (`cynic-host`, `cynic-kernel`, `cynic-node`).
2. **Functional Core, Imperative Shell (FCIS)**:
   - **Functional Core**: Domain modules, parsers, state machines, and reconciliation planners must be 100% pure (zero `std::process`, `tokio::spawn`, or network I/O).
   - **Imperative Shell**: Side-effects must reside exclusively behind explicit trait ports within `adapters/`.
3. **No Unproven Mutation**: Every mutation (host, network, source) must be bounded, explicit (`--yes`/`--apply`), and accompanied by machine-readable read-back proof.
4. **Zero Embedded Secrets**: Credentials, tokens, and private keys must never be committed to Git, configuration files, or tests. Secrets are injected at runtime behind `SecretProviderPort`.
5. **Language Invariant**: 100% of the codebase, tests, docstrings, schemas, and commit messages must be in English.

---

## Development Workflow

### 1. Branching Model

- **Features / Fixes**: `feature/<name>` or `fix/<name>`.
- **Automated Agents**: `agent/<agent-id>/<scope>`.
- **Direct commits to `main` are strictly prohibited.**

### 2. Commit Message Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>(<scope>): <short summary in English>

[optional body explaining intent, rationale, and proof]
```

Allowed types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`.

### 3. Pre-Submission Checklist

Before opening a pull request, run the following verification checks:

```bash
# 1. Format check
cargo fmt --all -- --check

# 2. Workspace compilation
RUSTFLAGS="-C debuginfo=0" cargo check --workspace

# 3. Secret scan (mandatory)
cynic audit secret --json

# 4. Architecture boundary check
cynic audit crate --json
```

---

## Pull Request Guidelines

1. **Focused Slices**: Submit small, reviewable pull requests addressing a single cohesive concern.
2. **Explicit Evidence**: Detail the commands executed and include test output or read-back proof in the pull request description.
3. **No Phantom Authorities**: Do not cite non-executable prose or unmerged documents as authority. Code must cite contracts (`contracts/`), types, or tests.
4. **Clean Git History**: Rebase against the target branch before opening the PR. Avoid merge commits in feature branches.
