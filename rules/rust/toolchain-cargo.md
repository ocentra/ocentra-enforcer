# Rust Toolchain And Cargo Gate Rules

Use this doc for workspace readiness, Cargo toolchain files, rustfmt, clippy,
tests, docs, and manifest metadata.

## Covered Rules

- `RR-1.1`: `rust-toolchain.toml` must pin the toolchain.
- `RR-1.2`: `Cargo.lock` must exist for deterministic builds.
- `RR-1.3`: `clippy.toml` must exist when workspace-file enforcement is enabled.
- `RR-1.4`: `deny.toml` must exist when workspace-file enforcement is enabled.
- `RR-1.5`: package or workspace package metadata must declare `rust-version`.
- `RR-10.1`: `cargo fmt` must pass.
- `RR-10.2`: hard-mode `cargo clippy` must pass.
- `RR-10.3`: `cargo test` must pass.
- `RR-10.4`: rustdoc warnings must be errors when doc gates are enabled.

## Agent Rule

Use file/crate scope while editing. Use workspace cargo gates only for readiness
or protected-branch validation.
