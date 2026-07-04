# Research Grounding — Design Principles and Citations

This document cites the research and design references that inform the Enforcer's architecture and rule mechanization approach. Every claim in the README's **Research Grounding** section traces back to a numbered source below.

---

## Cited Research and Design Sources

### 1. Context Budget Constraints for AI Task Performance

**Source:** Work on token limits, context window saturation, and task-specific routing in LLM-driven systems (e.g., ADBP doctrine, Anthropic guidance on managing Claude's 200K context window).

**Application:** The Enforcer routes agents to only the rule docs needed for touched files, scope, profile, or explicit rule ID (ref: README §2, "Indexed Decision Trees Save Context"). Long plans, AGENTS files, and rulebooks are indexed, not streamed by default, to preserve context budget for the actual work.

**Implementation:** `rules/INDEX.md`, `rules/rules.json` (machine-readable registry), and the `ocentra_enforcer_route` MCP tool emit a scoped decision tree. Agents read a small index first, classify the task, then open only the docs that apply. Fallback to broad reading only when the route is unknown or policy changes.

---

### 2. AST-Over-Prose Enforcement — Structural Validation as the Primary Gate

**Source:** Compiler design, type systems, and the principle of "invalid states unrepresentable" (e.g., Rust's type system, branded newtypes, parse-at-boundary validation). Applied to policy: rules expressed as structured data + mechanical validators, not prose rules backed by hope.

**Application:** Hard validators reject source slop, architecture drift, policy bypasses, weak tests, dependency issues, and secrets (ref: README §1, "Hard Gates Over Trust"). Every enforced rule has a dual shape: indexed rule docs explain what to do, and validators decide whether work is accepted.

**Implementation:** 
- Branded domain types (`enforcer-domain` crate with RuleId, RepoRoot, Sha256, etc. as `newtype` wrappers with serde validation).
- Parse-at-boundary: JSON/TOML config is decoded to typed structures at entry points; invalid schemas are rejected at compile/decode time, not silently defaulted.
- Mechanical validators emit the same `ruleId` as the docs name (ruleId ↔ validator ↔ doc ↔ fixture parity).
- If docs and validators disagree, the hard gate wins; fix code, docs, or strengthen the validator; do not bypass or weaken.

---

### 3. Ratchet Constraints — Baseline Enforcement with Non-Regressing Limits

**Source:** The ratchet pattern for metric-driven enforcement: once a measurement is baselinded, future runs must not exceed that value. Applied to scope creep, surface area, and cyclomatic complexity (e.g., cloc ratchets in CI, module-size caps, test-to-code ratios).

**Application:** The Enforcer's baseline ratchet (`d02` workpack) prevents silent expansion of findings, code size, or context surface. Violations cause CI to fail. A grown count fails; new findings fail; a ratchet cannot silently expand.

**Implementation:** 
- Baseline snapshot stored in `.enforce/baseline.json` (initial scan).
- Subsequent scans compared against baseline; any delta over the allowed tolerance fails CI.
- Composes with size/shape caps (d22: 200-line files, 30-line functions, 5-param signatures).
- Prevents projects from drifting into technical debt by claiming "we always had this problem."

---

### 4. Deferred-Work Gate — Explicit `DEFERRED(#ref)` Markers for TODO Discipline

**Source:** The principle that untracked technical debt metastasizes; explicit deferral with issue references and revisit checkpoints (common in production codebases: `TODO(#123)` with linked issues, revisit dates, and explicit abandonment decisions).

**Application:** The Enforcer's deferred-work gate (`d03` workpack) requires that any `TODO`, `HACK`, or `XXX` comment must include a valid issue reference (e.g., `DEFERRED(#ref)[revisit:date]`). Unmarked stubs in diffs fail.

**Implementation:** 
- Mechanical scan for `// TODO:` (etc.) without a `(#<issueId>)` reference in the matched line.
- Passes legacy stubs (comments pre-dating the gate).
- Fails if a diff introduces an unmarked stub or a malformed `DEFERRED(...)` marker.
- Enforces ownership: a defer is not a silent pass; someone must own the issue.

---

### 5. Rules-as-Structured Data — Declarative Rule Registries

**Source:** The shift from prose configuration files to schema-driven policy: Kubernetes manifests over shell scripts, Terraform HCL over readme docs, JSON schema over free-form comments. Applied to rule enforcement: rules are declared in a structured registry, not inlined prose.

**Application:** The Enforcer's rule registry (`rules/rules.json`) is machine-readable. Each rule is a structured object with `ruleId`, `title`, `framework`, `tier` (T1/T2/T3), `doc` (anchor), and associated validator/fixtures. Agents and MCP tools consume this registry to route relevant docs, explain findings, and re-run validators.

**Implementation:** 
- `rules/rules.json` is the single source of truth for rule metadata (not duplicated in prose or validator code).
- Every rule MUST have a registry entry, a rule doc (with the named anchor), a validator export, pass+fail fixtures, and a detection test (d01 parity oracle).
- The `/plan` skill and `enforcer:self` mechanization consume the registry to validate that declared rules match implemented validators.
- Invalid registry entries (unknown id, missing anchor, orphan fixtures) are caught by `d01` parity checks before a rule is considered "enforced."

---

## Research Grounding in Practice

These five foundations — context budgets, AST-over-prose, ratchets, deferred-work markers, and structured rules — are **not philosophy**. They are mechanically enforced:

- **Context budgets** are measured by the `ocentra_enforcer_route` tool, not hoped for.
- **AST-over-prose** is achieved by validators that reject invalid types/schemas at the boundary.
- **Ratchets** are stored in `.enforce/` and compared in CI; a grown baseline fails the gate.
- **Deferred-work markers** are regex-scanned; unmarked TODOs fail a pre-commit hook.
- **Structured rules** are validated by the d01 rule-scaffold-parity oracle; orphan or inconsistent rules fail.

Every claim in the README's research-grounding section maps to one of these five sources and their enforcement mechanisms. When the enforcer runs, these are not suggestions; they are hard gates backed by mechanical validators and CI gates.

---

## Integration with the Enforcer's Rust Crates

The structured rule registry and mechanical validators are built into the `enforcer-mechanization` and `enforcer-validator` crates. The rule scaffolder (`d01`) uses the registry to emit boilerplate for new rules and to verify that ruleId ↔ doc ↔ validator ↔ fixture parity is maintained. The coordination hub (arc-16) logs every rule invocation so ratchets can be audited and rule coverage can be measured.

---

*This grounding document was authored on 2026-07-04 as part of workpack d15 ("Readme Research Grounding"). It establishes the factual basis for design claims in the README and the doctrines described in [RUST_ARCHITECTURE.md](./plans/enforcer-selfhost-plan/RUST_ARCHITECTURE.md) and [EXECUTION_MODEL.md](./plans/enforcer-selfhost-plan/EXECUTION_MODEL.md).*
