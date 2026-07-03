# Ocentra Literal Scan

`ocentra-literal-scan` is a fast deterministic string literal risk scanner intended to be wired into Ocentra Enforcer.

It does **not** mean “ban all strings.” It separates:

- **Hard findings**: secrets and configured high-risk literals that can fail a gate.
- **Soft literal risks**: suspicious runtime literals that AI/humans should inspect.
- **Non-code text**: Markdown, JSON, TOML, YAML, ENV, and similar text/config formats are not literal-risk scanned as code. They should be handled by common security/config/doc checks.

The scanner is written in Rust with no third-party dependencies so it can compile offline. It uses deterministic language-family lexers and a broad language registry rather than a one-off Rust/TS/Python scanner.

## Commands

```bash
cargo test --manifest-path Tools/ocentra-literal-scan/Cargo.toml
cargo run --manifest-path Tools/ocentra-literal-scan/Cargo.toml -- scan --root . --json
cargo run --manifest-path Tools/ocentra-literal-scan/Cargo.toml -- scan --root . --files src/domain/user.rs --json
cargo run --manifest-path Tools/ocentra-literal-scan/Cargo.toml -- scan --root . --min-score 70 --json
cargo run --manifest-path Tools/ocentra-literal-scan/Cargo.toml -- scan --root . --fail-above 90 --json
```

## Output buckets

```json
{
  "ok": true,
  "summary": {
    "filesDiscovered": 128,
    "filesScanned": 40,
    "literalsFound": 900,
    "literalRisks": 22,
    "hardFindings": 0
  },
  "hardFindings": [],
  "literalRisks": []
}
```

Hard findings can fail. Literal risks are advisory unless `--fail-above` is passed or Ocentra Enforcer later maps a category to a hard profile.

## Built-in language registry

The v1 engine includes lexing for these language families:

- Custom: Rust, TypeScript/JavaScript, Python.
- C-like: C, C++, C#, Java, Kotlin, Scala, Go, Swift, Dart, PHP, Objective-C, Zig, D, V, Haskell-like basic strings, OCaml/F#/Erlang-like quoted strings, Solidity, Move, Apex, QML, CUDA, shaders.
- Hash-comment dynamic: Ruby, Perl, R, Julia, Nim, Elixir, Raku, Starlark-style files.
- Shell: sh, bash, zsh, fish, PowerShell, Batch, Makefile, Dockerfile.
- Lisp: Clojure, Common Lisp, Scheme, Racket, Emacs Lisp.
- Markup attributes: HTML, Vue, Svelte, Astro.
- Common text/config formats are recognized and skipped for literal-risk scanning.

Unknown textual files can be scanned with the fallback lexer by passing `--include-unknown-code`.

## Ignore behavior

By default it:

- respects `.gitignore`, `.ignore`, and `.git/info/exclude`;
- skips default generated/temp/log/cache directories such as `.git`, `target`, `node_modules`, `dist`, `build`, `coverage`, `.enforce`, `.ledger`, `output`, `test-results`, `playwright-report`, `.cache`, `tmp`, `temp`, `logs`, `.next`, `.turbo`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.venv`;
- skips common binary/artifact suffixes such as `.log`, `.tmp`, `.bak`, `.min.js`, `.map`, images, archives, audio/video, and databases;
- skips binary files and files above `--max-file-bytes`.

Use `--include-ignored` only for debugging.

## Risk categories

- `secret-like`
- `event-or-command-name`
- `route-or-url`
- `protocol-header-or-media`
- `id-or-key-name`
- `state-or-status`
- `raw-json-blob`
- `sql-fragment`
- `shell-fragment`
- `magic-string-comparison`
- `repeated-literal`
- `human-message`
- `test-fixture`
- `import-specifier`
- `schema-owner-literal`
- `unknown-literal`

Scoring is deterministic. No AI calls. No network. No hidden model.

## Integration

`integration/ocentra-literal-scan.mjs` is a minimal Node wrapper that Codex can copy into Ocentra Enforcer or call from the existing CLI. The intended wiring is:

```text
ocentra-enforcer advise literals --root . --files <paths...> --json
ocentra-enforcer scan --literal-risk summary|all
```

The Rust binary does the fast literal extraction and risk scoring. Node remains the policy/MCP/harness orchestrator.
