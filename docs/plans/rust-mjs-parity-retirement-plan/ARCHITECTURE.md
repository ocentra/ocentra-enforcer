# Parity and Retirement Architecture

## Behavioral comparison

```text
public frozen MJS at 267af94 ----+
                                    +--> identical fixture/input --> normalized observed contract
private overlay at 9d21780f9 -----+                                      |
                                                                           v
candidate native Rust at exact SHA --------------------------------> equal / stricter / gap
```

The public oracle supplies public behavior. The overlay supplies only its two exact private-test allowlist behaviors. The common base `d7162b617` records provenance and enables diff review; it is not a third pass authority. Native closure is the equal-or-stricter union of the required public behavior and the exact overlay additions.

## Capability row

Every registered capability row names:

- stable capability ID and owner;
- public MJS entrypoint plus exact oracle SHA;
- optional overlay behavior ID when one of the two exact additions applies;
- native CLI/MCP/library/CI entrypoint and candidate SHA;
- fixture/input/config/policy digests;
- scope expansion, exit/status, diagnostic ordering, side effects, and evidence contract;
- observed result: `equal`, `stricter`, `not-yet-native`, `legacy-only`, or `intentionally-retired`;
- artifact/run/CI identifiers and `doesNotProve`.

Schema equality is not behavior equality. Source presence is not runtime wiring. An unavailable oracle, timeout, malformed output, skipped required provider, docs-only CI run, or SHA mismatch leaves the row open.

## Replacement path

```text
inventory -> independent oracles -> boss gap adjudication -> bounded repairs
          -> exact-SHA aggregate -> clean-profile cutover rehearsal
          -> native release + observation -> delete-not-merge retirement
```

During measurement, MJS is invoked only as a pinned read-only test oracle. Candidate production routes must not delegate to Node and cannot count a Node-backed result as native. Rehearsal rollback returns to the previous native release, proving that production recovery does not depend on reviving MJS.

## Cutover surface

RM13 changes the selection atomically for public CLI, MCP registration, hooks, CI/action/workflow callers, installer/repair flows, dogfood, and release packaging. Before/after config bytes, installed artifact SHA, live session health, required CI SHA, observation window, and rollback evidence are retained.

## Retirement surface

RM14 deletes executable MJS enforcement entrypoints, wrappers, dependencies, tests that exist only for the retired runtime, and live CI/install references. It retains a bounded immutable provenance/evidence manifest. `safety-main` and the private overlay are deleted only after observation closes; they are never merged into `main` or `rust-build`.

## Parallel safety

Read-only oracle packets may run concurrently only for disjoint capability rows and artifacts. Repairs use isolated branches/worktrees and exact-file claims. Authority, capability matrix, registries, workflows, installers, aggregate proof, cutover, and retirement are serial boss/integrator surfaces.
