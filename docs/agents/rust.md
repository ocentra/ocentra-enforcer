# Rust Agent Persona

<!-- agent-capsule -->
> Agent Capsule
> Kind: T3 human-canonical prose (d09), advisory only.
> Read when: onboarding to Rust-stack conventions in this workspace.
> Stop rule: every must/never bullet below is checked by
> `crates/enforcer-validator/src/doc_rule_parity.rs`; do not add a bullet
> without a real `[ruleId]` citation — it will fail the doc-rule-parity
> gate.
<!-- /agent-capsule -->

Guidance for an AI agent (or human) writing Rust in this workspace. This is
advisory prose; the mechanized rule (linked via the `[ruleId]` on each
bullet) is the actual gate.

## Ownership and architecture

- Business logic must never live in `main.rs`; keep it thin and delegate to
  an application/domain layer [RUST-ARCH-1.1]
- The domain layer must never import forbidden I/O/framework crates
  [RUST-LAYER-1.1]
- Never introduce a catch-all `utils.rs`/`helpers.rs` dumping-ground module
  [RUST-NO-UTILS-MODULE]

## Ownership and safety

- Functions must borrow read-only parameters instead of taking ownership
  [RUST-BORROW-1.1]
- Every `unsafe` block must carry a `// SAFETY:` comment explaining why the
  invariant holds [RUST-SAFETY-COMMENT]
- Numeric casts must never use a lossy `as`; use `TryFrom` instead
  [RUST-CAST-NO-AS-LOSSY]

## Errors

- `main` must return `ExitCode`/`anyhow::Result<()>`; never scatter
  `process::exit` calls [RUST-ERR-MAIN-EXITCODE]
- Public error enums must never omit `#[non_exhaustive]` [RUST-ERR-NONEXHAUSTIVE]
- Never use a sentinel return value in place of `Result`/`Option`
  [RUST-ERR-SENTINEL]

## Style

- A `match` over a local enum must never carry a catch-all `_ =>` arm
  [RUST-MATCH-NO-WILDCARD]
- `#[allow(...)]`/`#[expect(...)]` must always carry a `reason = "..."`
  [RUST-ALLOW-1.1]

Free-text note: these bullets intentionally use "must"/"never" as the
imperative marker the doc-rule-parity gate looks for; do not add new
must/never language elsewhere in this file without a citation.
