# Common Literal-Risk Rules

## Covered Rules

- `SEC-2.10`: High-entropy secret assignments are forbidden.
- `LIT-1.1`: Low-confidence literals such as identifier, state, test fixture, schema-owner, import-specifier, and unknown literals require review.
- `LIT-1.2`: Event and command-name literals require review.
- `LIT-1.3`: Route and URL literals require review.
- `LIT-1.4`: Magic string comparisons require review.
- `LIT-1.5`: Protocol header and media literals require review.
- `LIT-1.6`: Raw JSON blob literals require review.
- `LIT-1.7`: SQL fragment literals require review.
- `LIT-1.8`: Shell fragment literals require review.
- `LIT-1.9`: Repeated literals require review.

## Enforcement

Run:

```bash
ocentra-enforcer advise literals --root <repo> --files <changed-files>
ocentra-enforcer check literal-risk --root <repo> --files <changed-files>
```

The scanner is deterministic and classifies results into:

- hard findings: secret-like literals and any configured hard categories
- warnings: lower-confidence literal risks that should be reviewed but do not fail by default

Profiles can raise the bar with `literalRisk.failAbove` or `literalRisk.hardCategories` when a repo wants stricter behavior.

## Fails

- Secret-like literals are present in source, config, proof, or test artifacts.
- A configured hard category is present in the current scope.
- The scanner reports an internal error or invalid output.

## Passes

- Obvious placeholders, tests, and human-message strings stay low-risk.
- Markdown, JSON, TOML, YAML, and other common text/config files are handled by the common scanners instead of this literal-risk pass.
- Advisory findings remain visible without turning into a hard fail unless the profile says otherwise.

## Fix Recipe

1. Move repeated or protocol-sensitive strings into shared constants or schema-backed values.
2. Replace shell fragments with argv arrays and SQL fragments with typed builders.
3. Convert dangerous values into placeholders or secret manager references.
4. Re-run the literal-risk check before claiming the packet done.

## Validator

- scanner: `common/literal-risk`
- commands:
  - `ocentra-enforcer advise literals --root <repo> --files <changed-files>`
  - `ocentra-enforcer check literal-risk --root <repo> --files <changed-files>`
