# UL07 - Reuse-First Tool Adapter Contract

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL07 Reuse-First Tool Adapter Contract`
> Kind: architect-owned harness deepening workpack.
> Read when: UL00 inventory identifies existing tools and before any language/security packet invents process execution.
> Stop rule: deepen the existing `enforcer-harness`; do not create a second generic runner.
> Proves: allowlisted tools have typed availability, bounded execution, normalized diagnostics, and evidence.
> Does not prove: any specific tool or rule family is semantically complete.
> Proof rule: missing, wrong-version, malformed, timed-out, failed, and finding outcomes are all non-ambiguous.
<!-- /agent-capsule -->

- owns: additive `crates/enforcer-domain/src/harness_types.rs`, `crates/enforcer-harness/src/adapters/**`, additive `crates/enforcer-harness/src/execution.rs`, `crates/enforcer-harness/src/parsers.rs`, `crates/enforcer-harness/tests/tool_adapter_contract.rs`, `crates/enforcer-harness/tests/fixtures/tool_adapters/**`
- deps: `UL00`
- tier: `P0 execution policy, P1 adapter conformance`

> Owner class: Sol/architect and named `tool-adapter-integrator`; Luna may add one adapter fixture only after the contract lands.
> Batch limit: one shared contract and one real-tool pilot.

## Where We Are

`enforcer-harness` already executes commands without a shell, parses cargo/tsc/pytest/ESLint/Bandit/Pyright/SARIF/CFLint output, and records runs. Execution accepts an arbitrary command vector, has no declared timeout/output bound/version policy, and represents missing or malformed required tools as graceful skip warnings.

## Where We Want To Be

The existing harness becomes the only shared process/tool adapter runtime for developer analyzers and CyberSkills external engines. Policy distinguishes required, optional, and advisory tools. A required tool that is missing, misconfigured, wrong-version, timed out, or malformed cannot yield pass.

## Owns

- typed `ToolSpec`, availability/version/config policy, bounded invocation, result/evidence, and normalized diagnostic metadata;
- allowlisted executable/argument templates with no shell interpolation;
- adapter conformance fixtures and exactly one pinned real-tool pilot;
- no language router row, security-engine mapping, CI workflow, installer policy, or new process-runner crate.

## Objective

Turn mature external mechanics into reliable proof providers reusable at author-time, hooks, MCP, CI, and release.

## Requirement Checklist

- [ ] Working directory and input/output paths remain within the declared repository/artifact roots.
- [ ] Availability states include `available`, `missing`, `version-mismatch`, `misconfigured`, `timed-out`, `failed`, and `malformed-output`.
- [ ] Policy declares whether each non-available state blocks, warns, or is not applicable; required never silently skips.
- [ ] Wall time, output bytes, file count/recursion, exit-code semantics, environment exposure, and network/credential posture are bounded.
- [ ] Evidence retains tool/version/config digest, command template identity, input/tree SHA, run ID, exit code, output digest, and normalized diagnostics.
- [ ] Diagnostics retain tool rule ID, severity, file/span, message fingerprint, language identity, and source provider.
- [ ] Same adapter contract is callable from local CLI/MCP/hook/CI paths.
- [ ] Existing arbitrary `run` remains explicitly user-invoked harness behavior and cannot be mistaken for an allowlisted policy gate.

## Acceptance And Proof

Use fake executable fixtures for every state, path escape, output overflow, timeout, and exit policy. Then run one real allowlisted tool on a pinned fixture repository and retain exact command/version/config/input/output evidence. Run harness/domain tests, security/dependency gates, and Enforcer scan.

## Stop conditions

Stop if the design duplicates harness storage/parsers, allows shell interpolation, cannot terminate/bound a child, treats a required missing tool as clean, or needs language/security-specific policy in the generic contract.

## Parallel Ownership Notes

The contract/execution/parsers are singleton and serialized through `tool-adapter-integrator`. After freeze, disjoint adapter parser/fixture packets may run in parallel and submit immutable evidence.
