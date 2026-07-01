# SBOM Rules

## Covered Rules

- `SBOM-1.1`: SBOM and dependency metadata generation must complete when requested.
- `NPM-1.12`: npm package SBOM generation must complete for release and PR-ready evidence.

## Enforcement

Run:

```bash
ocentra-enforcer check sbom --root <repo> --output target/security
```

Use `--dry-run` to validate the command path without writing generated artifacts.

## Fails

- A release or PR-ready path lacks dependency metadata or SBOM output when required.
- SBOM generation writes uncontrolled artifacts into source without configuration.

## Passes

- SBOM output is generated into the configured artifact directory and can be uploaded by CI.
- Dry-run mode validates availability without mutating source.

## Fix Recipe

1. Install or configure the required SBOM generator.
2. Set the output directory in project config or command flags.
3. Run the SBOM check locally and in CI.

## Validator

- scanner: `common/sbom`
- command: `ocentra-enforcer check sbom --root <repo> --output <artifact-dir>`
