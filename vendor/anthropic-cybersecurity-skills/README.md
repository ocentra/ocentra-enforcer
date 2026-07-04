# Vendored: Anthropic-Cybersecurity-Skills

This directory is a **vendored copy** of a third-party open-source corpus, preserved inside the
ocentra-enforcer repository. It is a **source we mechanize** into our own rule format — not code that
runs as part of the enforcer.

The upstream project's own README is preserved verbatim alongside this file as
[`README.upstream.md`](./README.upstream.md).

---

## Credit & thanks

**Anthropic-Cybersecurity-Skills** by **mukul975 (Mahipal)** —
<https://github.com/mukul975/Anthropic-Cybersecurity-Skills>

- Author: Mahipal (GitHub `mukul975`, `mukuljangra5@gmail.com`) — see [`CITATION.cff`](./CITATION.cff).
- License: **Apache License 2.0** — see [`LICENSE`](./LICENSE) (preserved at the corpus root and, per
  upstream, copied into 816/817 skill directories).
- Version vendored: 1.1.0 (released 2026-03-21).
- Contents: 817 cybersecurity skills, each `skills/<name>/{SKILL.md, scripts/agent.py, [scripts/process.py],
  references/*.md, LICENSE}`, plus `mappings/` (MITRE ATT&CK / OWASP / NIST-CSF crosswalks + a MITRE
  ATT&CK Navigator layer) and `tools/validate-skill.py`.

Our sincere thanks to the author for releasing this corpus under a permissive license. This is
high-quality, structured, MITRE-mapped security knowledge that would take a large team to assemble.

> **Naming note (important, not a slight):** despite the repository title, this project is **not
> authored or endorsed by Anthropic**. It is an independent, community-authored corpus by mukul975. We
> preserve the original name only to accurately identify the source; nothing here should be read as
> implying Anthropic authorship.

---

## What we vendored, and why

We vendored the corpus for two reasons:

1. **Preservation.** Keep a pinned, offline, attributable copy of the exact source we built rules from,
   so our derived rules remain traceable and reproducible even if upstream changes or disappears.
2. **Source-for-mechanization.** This corpus is a SOURCE we convert into OUR enforcer format —
   deterministic T1 validators with fail/pass fixtures + detection tests, scored T2 matchers, T3
   labeled-prose how-tos, and (only where an engine is genuinely irreplaceable) optional python/CLI
   run-adapters.

The plan and rationale for that conversion live in:

- [`RUST_CONVERSION_ANALYSIS.md`](./RUST_CONVERSION_ANALYSIS.md) — the verdict, the Rust-convertible vs
  python-bound split, the T1/T2/T3/adapter rule-ability breakdown with proposed ruleIds, the MITRE/OWASP
  vocabulary plan, and the f05→g02 wiring.
- `docs/plans/enforcer-selfhost-plan/workpacks/h11-cyberskills-corpus-to-rust-rules.md` — the workpack
  that mechanizes the fundamental-logic skills into native Rust rules.
- `docs/plans/enforcer-selfhost-plan/workpacks/h12-cyberskills-python-adapters.md` — the workpack for the
  optional, graceful-skip python/CLI run-adapters (the irreplaceable-engine skills).

---

## EXCLUDED from the enforcer's own dogfood scan

This corpus is a **third-party vendored source, not our code.** It MUST NOT be scanned by the enforcer's
own self-host dogfood run. The rules we DERIVE from it are dogfooded through the normal pipeline; the raw
vendored artifacts are not.

- The vendored **Python** (`skills/**/scripts/*.py`, `tools/*.py`) is precisely the "vendored Python"
  the owner's doctrine says our dogfood must not drown in — its fundamental logic is being reimplemented
  in Rust, and anything genuinely python-lib-bound stays as an optional adapter that is ignored from the
  dogfood scan.
- **Requirement:** add `vendor/**` (or at minimum `vendor/anthropic-cybersecurity-skills/**`) to
  `ignoreFileGlobs` in `ocentra-enforcer.config.json`. As of vendoring, that config's `ignoreFileGlobs`
  does not yet list `vendor/**`; the Rust scanner already excludes it because `rust-rules.config.json`
  `rustRoots` are limited to `src`/`crates`/`tools` and this tree is not a rust root.

---

## Modifications

This vendored copy is preserved **unmodified** except for these additive files, which do not alter any
upstream content:

- `README.md` (this file) — our attribution and vendoring notes.
- `README.upstream.md` — the upstream README, preserved verbatim.
- `RUST_CONVERSION_ANALYSIS.md` — our mechanization analysis.

The **rules we derive** from this corpus (in the enforcer's own `rules/` + `src/validators/` +
`crates/` trees) are a **modified Derivative Work** under Apache-2.0: we reimplement the fundamental
detection logic in Rust and re-express the threat mappings in our format. Those derived files carry the
required attribution to the upstream author and note that they are derived and modified.
