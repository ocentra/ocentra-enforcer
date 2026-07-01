# Boundary And Architecture Rules

## Covered Rules

- `BOUND-1.1`: Boundary modules require `BOUNDARY-INVARIANT:` documentation.
- `BOUND-1.2`: Raw boundary input must be converted to domain types at the boundary.
- `BOUND-1.3`: Boundary modules cannot contain domain decisions.
- `BOUND-1.4`: Domain modules cannot import boundary modules.
- `BOUND-1.5`: Boundary modules require negative invalid-input tests.
- `BOUND-1.6`: Boundary raw type count is budgeted.
- `BOUND-1.7`: Boundary glob additions require a waiver or owner note.
- `BOUND-1.8`: Boundary files cannot be named `utils` or `helpers`.
- `BOUND-1.9`: Boundary DTOs cannot leak into domain signatures.
- `BOUND-1.10`: Boundary conversion functions must return typed errors.
- `ARCH-1.1`: Domain cannot import infrastructure.
- `ARCH-1.2`: Domain cannot import UI.
- `ARCH-1.3`: Domain cannot import database clients.
- `ARCH-1.4`: Domain cannot import HTTP clients or servers.
- `ARCH-1.5`: Domain cannot import adapters.
- `ARCH-1.6`: Generated code cannot depend on domain internals.
- `ARCH-1.7`: Production source cannot import test support.
- `ARCH-1.8`: CLI/main code must depend on the application boundary only.
- `ARCH-1.9`: Circular imports/modules are forbidden.
- `ARCH-1.10`: Import-boundary config requires tests.
- `ARCH-1.11`: Public API surface is budgeted.
- `ARCH-1.12`: Barrel/facade files require explicit profile permission.
- `ARCH-1.13`: Public facades can expose only stable APIs.
- `ARCH-1.14`: Internal modules cannot leak through public types.
- `ARCH-1.15`: Package/crate ownership files are required.

## Enforcement

Run:

```bash
ocentra-enforcer scan --root <repo> --languages common --files <changed-files>
ocentra-enforcer check architecture-policy --root <repo> --files <changed-files>
```

## Fails

- Domain source imports UI, database, HTTP, adapter, or infrastructure modules.
- Boundary files accept raw inputs without conversion, lack invariants, or leak DTOs.
- Public facades expose internal/unstable APIs.

## Passes

- Boundaries convert raw inputs and return typed domain values/errors.
- Domain modules stay independent of boundary, UI, infrastructure, database, and HTTP layers.
- Packages and crates have explicit ownership metadata.

## Fix Recipe

1. Move transport and raw parsing to boundary modules.
2. Move domain decisions to domain owners.
3. Expose public APIs through stable, owned facades only.
4. Add ownership files for package and crate roots.

## Validator

- scanner: `common/architecture`
- command: `ocentra-enforcer scan --root <repo> --languages common --files <changed-files>`
