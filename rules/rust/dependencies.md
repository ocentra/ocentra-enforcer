# Rust Dependency Policy Rules

Use this doc for `Cargo.toml`, `Cargo.lock`, `deny.toml`, dependency metadata,
crate ownership, and supply-chain checks.

## Covered Rules

- `RR-9.1`: dependency versions must not use `*`.
- `RR-9.2`: git dependencies are forbidden by default.
- `RR-9.3`: path dependencies are forbidden outside policy.
- `RR-9.4`: path dependencies and configured dependency ownership must stay within policy.
- `RR-9.5`: direct registry dependency requirements must not drift.
- `RR-9.16`: dependency versions must not use loose ranges like `>=1` or major-only specs.
- `RR-9.22`: GPL/AGPL package licenses are forbidden by default.
- `RR-9.25`: `Cargo.lock` must be current after manifest changes.
- `RR-9.30`: `build-dependencies` require `BUILD-DEPENDENCY-JUSTIFICATION:`.
- `RR-11.1`: `cargo-deny` must pass when required.
- `RR-11.2`: `cargo-deny` must be installed when required.
- `RR-11.3`: `cargo-audit` must pass when enabled.

## Agent Rule

Do not weaken dependency policy to unblock implementation. Fix manifests,
profiles, or dependency ownership deliberately.

## Fails

- Rust dependency checks are unavailable, skipped, or fail policy.
- Cargo manifests use forbidden dependency shapes or bypass configured deny/audit policy.

## Passes

- `cargo-deny`, `cargo-audit`, and manifest checks pass when enabled by profile.
- Dependency exceptions are narrow, owned, expiring waivers.

## Fix Recipe

1. Install the required Rust dependency tool or mark the capability unavailable explicitly.
2. Fix deny/audit findings or remove the dependency.
3. Re-run dependency checks through Enforcer.

## Validator

- scanner: `rust/dependencies`
- command: `ocentra-enforcer scan --root <repo> --files Cargo.toml Cargo.lock`
