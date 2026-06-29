# Rust Dependency Policy Rules

Use this doc for `Cargo.toml`, `Cargo.lock`, `deny.toml`, dependency metadata,
crate ownership, and supply-chain checks.

## Covered Rules

- `RR-9.1`: dependency versions must not use `*`.
- `RR-9.2`: git dependencies are forbidden by default.
- `RR-9.3`: path dependencies are forbidden outside policy.
- `RR-9.4`: path dependencies and configured dependency ownership must stay within policy.
- `RR-9.5`: direct registry dependency requirements must not drift.
- `RR-11.1`: `cargo-deny` must pass when required.
- `RR-11.2`: `cargo-deny` must be installed when required.
- `RR-11.3`: `cargo-audit` must pass when enabled.

## Agent Rule

Do not weaken dependency policy to unblock implementation. Fix manifests,
profiles, or dependency ownership deliberately.
