# Rust Imports And Module Shape Rules

Use this doc for `*.rs` files when the change touches imports, module exports,
facades, preludes, or crate public API shape.

## Covered Rules

- `RR-7.1`: wildcard imports are forbidden.
- `RR-7.2`: wildcard public re-exports are forbidden.
- `RR-7.3`: public re-exports must match project policy.
- `RR-7.4`: dumping-ground modules such as `utils`, `helpers`, `common`, `misc`, `shared`, or `stuff` are forbidden.
- `RR-7.5`: `build.rs` is forbidden by default unless the profile explicitly allows it.

## Agent Rule

For strict projects, remove `pub use` entirely. Do not replace it with another
barrel-like shim.

## Fails

- Rust modules use wildcard imports, public re-exports, dumping-ground modules, or unauthorized `build.rs`.
- Architecture aliases depend on a cargo subcommand that is not installed.

```rust
pub use crate::domain::UserId;
use crate::domain::*;
```

## Passes

- Callers import directly from owning modules and crate architecture remains explicit.
- Re-export policy is enforced by Enforcer CLI/MCP, not local one-off scripts.

```rust
use crate::domain::user_id::UserId;

pub fn load_user(id: UserId) -> UserRecord {
    UserRecord::load(id)
}
```

## Fix Recipe

1. Replace `pub use` with direct imports from owning modules.
2. Rename dumping-ground modules to concrete domain names.
3. Run the Rust import/module scanner on the touched files or crate.

## Validator

- scanner: `rust/imports-modules`
- command: `ocentra-enforcer scan --root <repo> --files <changed-rust-files>`
