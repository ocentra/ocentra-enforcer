# CyberSkills, Syntax, Graph, and External Engine Architecture

## Decision

Use two deep modules and one explicit rule layer:

```text
source files
  -> Universal UL02/UL03/UL04 (one grammar owner, normalized facts, parse honesty)
  -> native CyberSkills predicates
  -> typed Findings

repository facts
  -> enforcer-memory CodeGraph (cross-file relations only when required)
  -> repository-scoped CyberSkills predicates
  -> typed Findings

targets/artifacts
  -> Rust-owned offline artifact/native-simulation predicates
  -> typed Findings and explicit notProved

external/reference repositories and tools
  -> parity/reference fixtures only
  -> never a user installation or product completion dependency
```

Universal owns grammar ownership transfer (UL02), shared syntax extraction (UL03), and parse-honesty/fact contracts (UL04). CyberSkills is a consumer: it records required facts and consumes accepted interfaces. It does not extract parsers, own grammars, or modify `enforcer-syntax`.

## Shared syntax dependency

### Consumer contract

The target interface is small and language-neutral:

```rust
pub fn analyze(request: SyntaxRequest<'_>) -> SyntaxOutcome;

pub struct SyntaxRequest<'a> {
    pub path: &'a RelPath,
    pub source: ValidationSource<'a>,
    pub limits: SyntaxLimits,
}

pub enum SyntaxOutcome {
    Parsed(SyntaxFacts),
    Unsupported(UnsupportedLanguage),
    Invalid(ParseDiagnostics),
    ResourceLimited(ResourceLimit),
}
```

`SyntaxFacts` normalizes only reusable structural facts: symbols, calls and arguments, imports, assignments, literals, annotations/decorators, type relations, control-flow facts, bounded data-flow facts, and source spans. Capabilities may grow behind this interface, but security rules do not import grammar crates or raw Tree-sitter node kinds.

### Ownership rules

- UL02 records the sole grammar/runtime owner; UL03 owns grammar bindings, vendored grammar source, extension dispatch, ABI checks, and extraction; UL04 owns syntax facts and parse diagnostics.
- `enforcer-memory` and security crates consume the landed interface. CyberSkills may not add a grammar dependency, raw Tree-sitter node type, parser, or compatibility re-export.
- CyberSkills fact demand is accepted only after UL04 proves the required capability and explicit invalid/unsupported/resource-limited behavior.

### Parser selection matrix

| Input | Required mechanism |
|---|---|
| JSON/YAML/TOML with a stable typed schema | Serde/domain parser; Tree-sitter is unnecessary |
| HCL/Dockerfile/source language where comments, strings, calls, or nesting matter | `enforcer-syntax` |
| Log/signature matching where syntax has no semantic role | Deterministic matcher is acceptable |
| Cross-file call/import/type relation | `enforcer-memory` graph over `enforcer-syntax` facts |
| Dynamic, binary, forensic, network, symbolic, or live-cloud analysis | External engine |

Regex is not forbidden. It is forbidden to use regex as structural proof when comments, strings, nesting, aliases, or call arguments can change the verdict.

## Shared external/reference boundary

The existing `enforcer-harness` adapter contract remains available for recorded parity evidence and explicitly unavailable outcomes. It is not the product implementation path. CyberSkills owns the intent requirement, the Rust-native predicate/offline model, and the boundary that prevents an external tool result from becoming native coverage. CP06/CP07 are reference/parity workpacks, not a requirement that a customer install or pay for a third-party engine.

Universal UL07 deepens the shared adapter, recorded-evidence, and unavailable-outcome contract; it does not turn an external engine into native CyberSkills coverage.
CP06 does not create or modify the generic runner; it consumes the Universal UL07 contract.

```rust
pub fn run(request: EngineRequest, adapter: &dyn EngineAdapter)
    -> EngineRunEvidence;
```

The interface includes a typed engine ID, allowlisted executable/version constraint, typed target, declared network/credential policy, timeout, resource/output limits, config digest, and expected output schema. Evidence includes outcome, tool version, command/config digest, artifact hashes, normalized findings, stderr summary, and coverage limits.

Outcomes are distinct: `Ran`, `SkippedUnavailable`, `RejectedPolicy`, `TimedOut`, `Errored`, and `InvalidOutput`. Only `Ran` can satisfy an engine-required component. Optional absence may narrow coverage but cannot become a pass.

There is one adapter per real engine or stable output protocol, not per skill. Many skills may reference the same reference component, but an adapter result never becomes native coverage.

## Intent-family and packet layer

The 816 available catalog identities are classified once into 34 canonical intent families in `CYBERSKILLS_INTENT_MATRIX.json`. The graph creates stable family nodes and derives disjoint component packets:

- CP09 owns native static/offline packets, at most five skills.
- CP12 owns native repository-graph packets, one predicate at a time.
- CP11 owns advisory/manual retention packets, at most ten skills.
- The blocked external-engine component remains visible in each skill's truth but has no product implementation packet.

Each packet carries source-hash/anchor requirements, reuse references, exact component ownership, and `notProved`. Packet selection is derived by the graph; Markdown routing labels never create readiness.

## CyberSkill coverage model

Every skill record has stable source identity and one or more components when its source is available:

```text
skillId
sourcePath + sourceSha256 + sourceAvailability + attribution
components[]:
  componentId
  kind: native-predicate | external-engine | advisory | manual
  tier: T1 | T2 | T3
  status: proposed | implemented | proved | retained | blocked
  predicateOrPurpose
  implementationRef
  evidenceRefs[]
  notProved[]
```

`sourceAvailability` is `available` or `sourceUnavailable`. The one unavailable identity records tracked blob `df48fa4149dd25956e730443d3582693a3f825a8`, observed absence, and an owner-decision reference; it has no reviewed-source components and never contributes to coverage. All totals are derived. Hand-written totals and a single mutually exclusive disposition are rejected.

## Existing-rule policy

The 41 existing rules remain valid candidates. CP01 first proves their source mappings. CP04 then selects one high-value source-pattern rule for syntax migration. Rules already using the correct typed JSON/YAML/domain parser are not rewritten merely to use Tree-sitter. Text rules remain unchanged when their predicate is genuinely textual and their fixtures prove comment/string and malformed-input behavior.

## Cross-file graph policy

Graph use is opt-in and repository-scoped. A rule must show that a file-local predicate cannot prove the contract, declare traversal limits, and provide positive, negative, ambiguous, cycle, and resource-limit fixtures. It consumes the accepted Universal UL13 graph/provider contract. Knowledge-graph availability is an explicit outcome; it never silently changes a clean verdict.

## Attribution

The vendored repository name must not be treated as authorship. Preserve its LICENSE, CITATION, frontmatter, source paths, and hashes. Derived rule records cite the actual vendored source and state the narrowed predicate copied or re-expressed.
