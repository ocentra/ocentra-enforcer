# Verification

Run after extracting into a repository with Rust installed:

```bash
cargo test --manifest-path Tools/ocentra-literal-scan/Cargo.toml
cargo run --manifest-path Tools/ocentra-literal-scan/Cargo.toml -- scan --root Tools/ocentra-literal-scan/tests/fixtures/bad --json
cargo run --manifest-path Tools/ocentra-literal-scan/Cargo.toml -- scan --root Tools/ocentra-literal-scan/tests/fixtures/languages --json --include-low
```

Expected behavior:

- `tests/fixtures/bad` has one hard secret finding and several literal risks.
- `tests/fixtures/good` has no hard findings.
- `tests/fixtures/ignored` scans zero files by default because `.gitignore` and ignored directories are respected.
- `tests/fixtures/languages` scans broad language-family fixtures and skips Markdown/JSON as code literal-risk targets.

Note: this package was generated in an environment without `rustc`/`cargo`, so the included tests are authored but were not executed here. The code has no third-party crate dependencies and should compile offline with Rust 1.75+.
