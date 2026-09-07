# UL02 Grammar Ownership Decision

Status: accepted decision; ready for UL03 execution after this record is integrated.

## Authority and decision

The boss and the current language-parity custodian accepted this decision in both
roles in coordination event `evt_da17dc38e30e465c9730ba1d1466a162`.
The accepted choice is **G3-after-extraction**:

1. Freeze the existing grammar substrate at commit
   `3d3de6e63ecd07da7e2f980007dfcdce68df4fff` (tree
   `f042bab7c3e593fb77fdc99267c8d1a33830b98c`).
2. UL03 moves the substrate, without duplication, into the sole owner
   `crates/enforcer-syntax/**`.
3. After the move, execute G3 Stage 5 full C/Rust parity verification and
   regenerate parity proof from the new owner.

This record is the only UL02 write. It authorizes neither parser/grammar source
edits nor the UL03 extraction itself.

## Ownership at the migration boundary

At the accepted base, custody is the historical enforcer-selfhost
language-parity campaign under `rust-build` primary custody. The read-only audit
found no live language-parity task branch and no active claim on the grammar
surface.

The frozen source ownership patterns are:

- `crates/enforcer-memory/src/parsers/**`: parser dispatch, safe parser
  boundaries, and parsed-file construction;
- `crates/enforcer-memory/src/languages/**`: `Language`, `LangSpec`, language
  classification, generic extraction, and language-specific normalized facts;
- `crates/enforcer-memory/vendor/**`: vendored Tree-sitter bindings, generated
  parser/scanner sources, grammar metadata, and licenses;
- `crates/enforcer-memory/Cargo.toml`: the existing grammar/provider dependency
  declarations;
- `crates/enforcer-memory/src/lib.rs`: the memory-side module boundary and
  compatibility imports required by the move.

UL03 owns the destination `crates/enforcer-syntax/**` and only the transferred
paths named above. A single named integrator owns any workspace
`Cargo.toml`/`Cargo.lock` edits; memory `Cargo.toml` and `src/lib.rs` changes
are compatibility edits only. `enforcer-syntax` becomes the sole owner of
classification, grammar providers, parser dispatch, safe parser boundaries, and
normalized extraction. `enforcer-memory` consumes that API for graph,
persistence, and runtime behavior; neither crate owns enforcement policy.

## Evidence and live claim audit

The audit was read-only at the accepted base and used the source tree plus the
`enforcer-rust-build` coordination hub. It found:

- no active grammar/parser/vendor/workspace-manifest claim;
- no in-flight grammar commit or unresolved language-parity branch;
- no overlapping current-custodian claim requiring adjudication;
- unrelated claims remained out of scope: the frozen safety-main review and the
  CyberSkills CP08 decomposition proof path. They do not overlap this map.

The current source surfaces are the exact paths above, with existing language
fixtures under `crates/enforcer-memory/tests/unit_languages_*.rs`, parser
contracts under `crates/enforcer-memory/tests/property_parser_contracts.rs`,
and parity evidence under `crates/enforcer-memory/tests/feature_parity/**`,
`crates/enforcer-memory/tests/parity_architecture.rs`,
`crates/enforcer-memory/tests/parity_read_tools.rs`, and
`crates/enforcer-memory/tests/parity_trace_tools.rs`. These prove audit
coverage and existing test surfaces only; they do not prove post-move parity.

## Freeze and in-flight work

The freeze starts at the accepted base/tree above and ends only after UL03 has
completed the behavior-preserving move, the post-move dependency/import checks
are green, and G3 Stage 5 C/Rust parity proof has been regenerated from
`enforcer-syntax`.

During the freeze, no new grammar, parser, vendored asset, or workspace
manifest change is admitted. The sole exception is an emergency security or
parity fix explicitly approved by `rust-build` primary. Such a fix must be
recorded, replayed, and proven in both pre-move and post-move evidence before
the transfer can close.

Any grammar commit or claim appearing after the freeze is rejected by default;
it may enter the migration only through explicit primary admission naming the
commit, paths, owner, and updated evidence. There were no in-flight grammar
commits or claims at freeze start, so there is no grandfathered work.

## Ordered move and rollback

UL03 must perform this order:

1. preserve the accepted-base fixture and dependency inventories;
2. create `enforcer-syntax` and move the parser/language/vendor paths above;
3. update memory imports and its dependency edge through the named integrator,
   without a duplicate grammar registry or compatibility copy;
4. run the existing parser, language, generic, unsafe-input, graph-ingest, and
   memory fixtures before adding any new capability;
5. compare dependency graphs and fixture totals, then run G3 Stage 5 parity
   verification from the new owner.

If UL03 must be undone, use ordinary revert commits for the extraction and
compatibility changes. Preserve vendored bytes, licenses, and evidence; do not
use destructive reset, delete-and-recreate, or vendor loss as rollback.

## Acceptance gates and non-claims

Before UL03 begins, validate this record with plan/link checks, exact-file
Enforcer routing and scan, the coordination guard, and a read-only source/claim
inventory. UL03 remains blocked until this committed record is referenced from
both the Universal Language and CyberSkills parity plans by their authorized
integrators.

This decision proves the migration authority, base/tree selection, freeze
interval, ownership map, move order, claim audit, and rollback policy. It does
not prove parser parity, complete language enforcement, G3 Stage 5 completion,
successful extraction, or any UL03 source change.
