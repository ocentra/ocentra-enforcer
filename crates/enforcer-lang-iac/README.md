# enforcer-lang-iac

Per-family `Validator` implementations for the infrastructure-as-code rule
family (`IAC-1.1` .. `IAC-1.8`, 8 rules): Terraform (HCL), CloudFormation
(JSON/YAML templates), and Kubernetes manifest config-shape checks.

Built on [`enforcer-validator`](../enforcer-validator)'s `Validator` trait
and `run_fixture_parity` harness — the same pattern `enforcer-lang-ts`,
`enforcer-lang-py`, `enforcer-lang-rust`, and `enforcer-lang-common` use.

## Rule families

| Module | Rules | Shape |
|---|---|---|
| `rules::terraform` | IAC-1.1, IAC-1.2, IAC-1.3, IAC-1.6, IAC-1.7 | S3 encryption, open ingress, hardcoded secrets, provider version pin, remote-state encryption |
| `rules::cloudformation` | IAC-1.4, IAC-1.5 | S3 public-access-block, IAM wildcard action+resource |
| `rules::kubernetes` | IAC-1.8 | privileged container |

## Two detection shapes

`rules::spec::TriggerKind` covers two shapes IaC rules need beyond the
single forbidden-pattern-present model `enforcer-lang-ts` uses:

- `ForbiddenPresent` — a forbidden token appears anywhere (hardcoded
  secret, open CIDR, `privileged: true`). Fires per line.
- `RequiredAbsent` — a scoping resource/block type is present but a
  required companion key is absent anywhere in the file (an
  `aws_s3_bucket` with no `server_side_encryption_configuration`; a
  `required_providers` block with no `version`; a `backend "s3"` block
  with no `encrypt`). Fires once per file, anchored at the scope line.

## External-engine seam

This crate validates static IaC text from Rust; it does not execute
`terraform`, `cfn-lint`, or `cflint`. Where a real external engine would
add value beyond static text checks, that integration is a
`enforcer-harness` graceful-skip adapter concern, not this crate's.

## Proof

```
cargo test -p enforcer-lang-iac
```

Every one of the 8 rule ids fires on its `fail` fixture and stays silent
on its `pass` fixture (`run_fixture_parity`); `tests/completeness.rs`
proves the registry has exactly the 8 `language == "iac"` rows from
`rules/rules.json`, no orphans, no duplicates.
