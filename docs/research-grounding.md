# Engineering Foundations

Ocentra Enforcer is designed around a small set of practical engineering
principles. They describe the product's behavior; the executable rules,
fixtures, and tests remain the authority for what is enforced.

## Deterministic acceptance

Policy that matters should be evaluated by a validator. Documentation explains
the expected result, while typed rules, scoped checks, and CI decide whether a
change is accepted.

## Scoped work

Routing and scope selection keep work proportional to the change. A file or
crate check is useful during implementation; a workspace or CI check confirms
the wider result before integration.

## Structured diagnostics

Tools retain raw artifacts when needed, but the normal interface is a compact
structured finding with a stable rule identifier, location, severity, and
repair context. This makes failures actionable for people and automation.

## Evidence over assertion

Claims about correctness should be backed by a reproducible test, command,
artifact, or proof record. Evidence must identify what ran and the revision it
applies to.

## Improvement without silent weakening

When a check exposes a real problem, the preferred response is to repair the
code, policy, fixture, or boundary. Broad suppressions and untracked exceptions
hide useful signal and are not a substitute for a valid fix.
