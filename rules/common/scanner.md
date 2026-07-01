# Scanner Contract Rules

## Covered Rules

- `SCAN-1.1`: Scanners must ignore patterns inside string literals where appropriate.
- `SCAN-1.2`: Scanners must ignore patterns inside comments where appropriate.
- `SCAN-1.3`: Scanners must still detect suppression comments.
- `SCAN-1.4`: Scanners handle CRLF and LF identically.
- `SCAN-1.5`: Scanners handle Unicode paths.
- `SCAN-1.6`: Scanners handle spaces in paths.
- `SCAN-1.7`: Scanners handle Windows drive paths.
- `SCAN-1.8`: Symlink policy must be explicit.
- `SCAN-1.9`: Scanners must avoid symlink loops.
- `SCAN-1.10`: Scanner output is sorted by normalized path, line, rule ID, and message.
- `SCAN-1.11`: Scanner file reads are bounded.
- `SCAN-1.12`: Binary files are skipped safely.
- `SCAN-1.13`: Invalid input produces bounded diagnostics, not crashes.
- `SCAN-1.14`: Unknown extensions do not trigger false language scans.
- `SCAN-1.15`: Diff scope handles deleted/renamed files deterministically.
- `SCAN-1.16`: File scope does not accidentally scan the whole repo.
- `SCAN-1.17`: Workspace scope scans configured roots.
- `SCAN-1.18`: Crate/package scope resolves owning manifests.
- `SCAN-1.19`: Scope reports include included/excluded file counts.
- `SCAN-1.20`: Doctor output exposes ignore globs.
- `SCAN-2.1`: Rust strict mode uses `cargo metadata`.
- `SCAN-2.2`: Rust strict mode uses parser-backed checks where regex is unsafe.
- `SCAN-2.3`: Rust strict mode ingests Clippy JSON.
- `SCAN-2.4`: Rust strict mode ingests rustdoc warnings.
- `SCAN-2.5`: TypeScript strict mode uses TypeScript or ESLint JSON.
- `SCAN-2.6`: Python strict mode ingests Ruff JSON.
- `SCAN-2.7`: Python strict mode ingests Pyright or mypy output.
- `SCAN-2.8`: Common security strict mode ingests SARIF.
- `SCAN-2.9`: Regex scanners remain fast preflight.
- `SCAN-2.10`: Native and regex reports merge without duplicate spam.

## Fails

- A file-scope scan walks the full repository.
- A scanner follows a symlink loop.
- Unknown extensions are treated as Rust, TypeScript, or Python source.
- Native diagnostics and regex diagnostics duplicate the same finding repeatedly.
- CRLF and LF produce different line numbers.

## Passes

- Path traversal uses normalized paths and skips symlinks explicitly.
- File, diff, crate/package, and workspace scopes are separate code paths.
- Fast regex scanners catch cheap structural issues before native tool ingestion.
- Native JSON/SARIF diagnostics are normalized and deduplicated through the harness.
- Scope metadata tells agents what was included and what was excluded.

## Fix Recipe

1. Mask strings/comments before structure-only regex checks.
2. Keep suppression-comment checks separate so disables are still detected.
3. Resolve scope entries before walking files.
4. Skip symlinks and binary files explicitly.
5. Send native tool output through the harness normalization path.

## Validator

- scanner: `common/scanner-contracts`
- implemented in: `src/checks.mjs`, `src/path-utils.mjs`, `src/source-policy-scanners.mjs`, and `src/harness.mjs`
- command: `ocentra-enforcer check scanner-contracts --root <repo>`
