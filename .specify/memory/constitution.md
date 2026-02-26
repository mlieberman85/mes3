<!--
  Sync Impact Report
  ==================
  Version change: (none) → 1.0.0 (initial ratification)

  Added principles:
    - I. Test-First (NON-NEGOTIABLE)
    - II. Simplicity
    - III. Security-by-Default
    - IV. Observability
    - V. Semantic Versioning

  Added sections:
    - Core Principles (5 principles)
    - Technology Constraints
    - Development Workflow
    - Governance

  Removed sections: (none — fresh constitution)

  Templates requiring updates:
    - .specify/templates/plan-template.md        ✅ compatible (Constitution Check section is generic)
    - .specify/templates/spec-template.md         ✅ compatible (no constitution-specific references)
    - .specify/templates/tasks-template.md        ✅ compatible (phase structure aligns with principles)
    - .specify/templates/agent-file-template.md   ✅ compatible (no constitution references)
    - .specify/templates/commands/                ✅ N/A (directory empty)

  Follow-up TODOs: none
-->

# mes3 Constitution

## Core Principles

### I. Test-First (NON-NEGOTIABLE)

All new functionality MUST have tests written and approved before
implementation begins. The Red-Green-Refactor cycle is strictly
enforced:

- Tests MUST be written first and confirmed to fail.
- Implementation MUST target passing those tests and nothing more.
- Refactoring MUST NOT change observable behavior; tests MUST
  remain green after every refactor step.
- Untested code MUST NOT be merged into the main branch.

**Rationale**: Tests encode requirements as executable contracts.
Writing them first prevents scope creep and ensures every line of
production code has a reason to exist.

### II. Simplicity

Every design decision MUST favor the simplest viable solution.

- YAGNI: features, abstractions, and configuration MUST NOT be
  added until a concrete, present-day requirement demands them.
- Abstractions MUST be justified by at least two distinct callers;
  a single use case does not warrant indirection.
- Complexity MUST be documented with a rationale explaining why
  a simpler alternative was rejected.

**Rationale**: Complexity is the primary enemy of maintainability.
Keeping the codebase small and direct reduces onboarding cost,
bug surface, and cognitive load.

### III. Security-by-Default

All code MUST be secure in its default configuration.

- User input MUST be validated at system boundaries before any
  processing occurs.
- WASM modules MUST operate within sandboxed memory; no raw
  pointer access across module boundaries.
- Dependencies MUST be audited for known vulnerabilities before
  adoption and on every update.
- Secrets and credentials MUST NOT appear in source code,
  configuration files, or build artifacts.

**Rationale**: A Rust/WASM tool running in browsers inherits the
web threat model. Secure defaults eliminate entire classes of
vulnerabilities without requiring downstream vigilance.

### IV. Observability

System behavior MUST be inspectable at runtime without source
code access.

- All significant operations MUST emit structured log entries
  (key-value or JSON format).
- WASM modules MUST expose diagnostic hooks the frontend can
  query (e.g., version, configuration, processing state).
- Errors MUST propagate with enough context to identify root
  cause without reproducing the failure.

**Rationale**: A WASM+frontend architecture makes traditional
server-side debugging unavailable. Built-in observability is the
primary diagnostic tool for both developers and users.

### V. Semantic Versioning

All published interfaces MUST follow MAJOR.MINOR.PATCH
versioning (SemVer 2.0.0).

- MAJOR: backward-incompatible API or WASM interface changes.
- MINOR: backward-compatible feature additions.
- PATCH: backward-compatible bug fixes.
- Breaking changes MUST include a migration guide and a
  deprecation period of at least one minor release when feasible.

**Rationale**: The WASM module is consumed by the frontend (and
potentially by third parties). Predictable versioning prevents
silent breakage across the boundary.

## Technology Constraints

- **Language**: Rust (latest stable toolchain). All core logic
  MUST compile to `wasm32-unknown-unknown`.
- **Frontend**: Web-based UI that loads and drives the WASM
  module. Framework choice MUST be documented in the feature
  plan before implementation begins.
- **Build targets**: The project MUST produce both a native
  binary (for testing/CLI usage) and a WASM artifact.
- **Dependencies**: Prefer `no_std`-compatible crates for WASM
  core logic. Each new dependency MUST be justified in the PR
  description.
- **Browser support**: The frontend MUST function in the latest
  stable releases of Chrome, Firefox, and Safari.

## Development Workflow

- **Branching**: All work MUST happen on feature branches.
  Direct commits to `main` are prohibited.
- **Code review**: Every pull request MUST receive at least one
  approval before merge.
- **CI gates**: PRs MUST pass the following before merge:
  1. `cargo test` (native)
  2. `cargo test --target wasm32-unknown-unknown` (WASM)
  3. `cargo clippy -- -D warnings`
  4. `cargo fmt --check`
  5. Frontend lint and test suite
- **Commit messages**: MUST follow Conventional Commits format
  (`type(scope): description`).
- **Documentation**: Public APIs and WASM-exposed functions MUST
  have doc comments. User-facing changes MUST update relevant
  documentation before merge.

## Governance

This constitution is the supreme authority for project practices.
When a conflict exists between this document and any other guide,
this document prevails.

- **Amendments** require:
  1. A written proposal describing the change and its rationale.
  2. Review and approval by at least one maintainer.
  3. A migration plan if existing code must change to comply.
  4. An updated constitution version following SemVer rules.
- **Compliance reviews** MUST occur at the start of every feature
  plan (see the Constitution Check gate in plan-template.md).
- **Violations** discovered in review MUST be resolved before
  merge or explicitly documented with justification in the
  Complexity Tracking table.

**Version**: 1.0.0 | **Ratified**: 2026-02-25 | **Last Amended**: 2026-02-25
