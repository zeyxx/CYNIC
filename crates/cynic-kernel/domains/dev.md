# Dev Domain — Axiom Evaluation Criteria

Evaluate the CODE CHANGE, COMMIT, or ARCHITECTURAL DECISION described — not the quality of its textual description. A brilliant refactoring poorly described is still brilliant engineering. A regression described eloquently is still a regression.

## FIDELITY
Is this faithful to sound engineering principles? Does it follow established patterns, correct error handling, and proven design?
- HIGH (0.55-0.65): Follows SOLID principles, correct error handling, well-tested, addresses root cause. Example: extracting a shared function to eliminate duplication (DRY) with tests covering both callers.
- MEDIUM (0.25-0.45): Reasonable approach but minor issues — missing edge case, slightly over-engineered, acceptable but not ideal. Example: adding a retry loop without backoff.
- LOW (0.05-0.20): Violates fundamental principles — silent error swallowing, hardcoded secrets, untested critical path, copy-paste duplication. Example: `unwrap()` on user input in production code.

## PHI
Is this structurally harmonious? Are components well-proportioned? Is the abstraction level consistent?
- HIGH (0.55-0.65): Clean separation of concerns, consistent abstraction level, interfaces match responsibilities. Example: a port trait with one adapter, tested at the boundary.
- MEDIUM (0.25-0.45): Mostly clean but some leaky abstractions or mixed levels. Example: a domain function that imports an infra type but isolates it well.
- LOW (0.05-0.20): God objects, mixed concerns, abstraction mismatch. Example: a 500-line function mixing HTTP parsing, business logic, and database queries.

## VERIFY
Is this testable? Are claims verifiable? Can it be falsified?
- HIGH (0.55-0.65): Has tests, tests are meaningful (not tautological), covers edge cases, includes before/after measurement. Example: TDD — failing test written first, minimal implementation, then refactored.
- MEDIUM (0.25-0.45): Has some tests but gaps — happy path only, or tests exist but don't assert meaningful properties. Example: test that checks the function doesn't panic but not the output.
- LOW (0.05-0.20): Untested, untestable (tightly coupled), or claims "works" without evidence. Example: "I tested it manually" with no automated test.

## CULTURE
Does this honor the project's patterns and conventions? Does it respect the codebase's idioms?
- HIGH (0.55-0.65): Follows existing patterns exactly, naming consistent, file placement matches convention. Example: new backend follows the OpenAiCompatBackend pattern — same trait, same error handling, same test structure.
- MEDIUM (0.25-0.45): Mostly follows convention but introduces minor deviations. Example: using a different error type than the rest of the codebase.
- LOW (0.05-0.20): Ignores project conventions, introduces foreign patterns, breaks existing idioms. Example: adding a Python script to a Rust project for something the Rust toolchain handles.

## BURN
Is this efficient? Minimal waste? Does every line justify its existence?
- HIGH (0.55-0.65): Minimal code, no dead paths, no speculative abstractions, solves exactly the stated problem. Example: a 20-line function that replaces 200 lines of over-engineered code.
- MEDIUM (0.25-0.45): Reasonable but some waste — unnecessary abstractions, over-engineering for hypothetical futures. Example: adding a config option for something that has exactly one value.
- LOW (0.05-0.20): Bloated, dead code, premature abstractions, solves problems that don't exist. Example: a factory-builder-strategy pattern for a single implementation.

## SOVEREIGNTY
Does this preserve agency and independence? Does it avoid vendor lock-in? Does it keep control local?
- HIGH (0.55-0.65): Sovereign infrastructure, no new external dependencies, data stays local, can be modified freely. Example: implementing a feature using only the project's existing dependencies.
- MEDIUM (0.25-0.45): Introduces a dependency but behind a port trait, or uses a well-maintained open-source library. Example: adding a crate with an abstraction layer.
- LOW (0.05-0.20): Hard vendor lock-in, proprietary dependencies, data leaves the system, cannot be replaced. Example: calling a cloud API directly from domain code with no abstraction.
