# Architecture

## Deep module shape

```text
Tree-sitter bindings and LangSpec tables
                |
                v
        enforcer-syntax
  classify + parse + normalized facts
        /                     \
       v                       v
enforcer-memory            enforcer-scan
graph/persistence        parse once per file
                               |
                               v
                    enforcer-validator input
                    raw source + typed facts
                               |
             +-----------------+-----------------+
             v                 v                 v
       language rules     security rules    graph/external rules
```

`enforcer-syntax` is the sole grammar/runtime owner after UL02/UL03. It contains no memory store, embedding, model, UI, coordination, or rule policy. `enforcer-memory` consumes it for graph construction. `enforcer-scan` consumes it for parse-once analysis. Validators see normalized domain facts, never Tree-sitter nodes.

## Fact boundary

The fact model must add, in bounded slices:

- language/file identity and parser provider/version;
- parse outcome, error/missing-node counts, unsafe-input refusal, and provenance;
- stable byte/line spans and normalized node/fact kinds;
- existing symbols, routes, imports, calls, inherits, implements, decorates, type references, and defines;
- later capability slices for parameters, return/type annotations, literals, assignments, containment, visibility/modifiers, and bounded control/data-flow.

UL04 does not promise every fact for every language. It defines completeness per language and prevents empty/default output from masquerading as a successful parse.

## Validator compatibility

Extend the validator seam without forcing every text rule to become AST-backed in one migration. Existing validators retain a raw-source path. Fact-backed validators declare required capabilities and receive a prepared analysis input. Scan owns parse caching and dispatch. Missing required facts become explicit diagnostics.

## Taxonomy consolidation

One canonical language registry becomes the source for parser identity, literal lexing, scan routing, UI/MCP schemas, and generated capability reports. Coarse families remain derived dispatch groupings, not competing language truth.

## Framework adapters

Effect/Zod/Valibot/Pydantic/etc. adapters recognize normalized boundary-shape evidence. They are fact producers selected by language/ecosystem, not separate universal doctrines. The doctrine resolver decides whether a recognized family satisfies the active profile.

## Graph and external engines

Cross-file rules consume a typed graph interface, not the memory database. Deep compiler/linter/security semantics use the shared allowlisted `enforcer-harness` adapter contract deepened by UL07. This plan owns that shared adapter contract; CyberSkills consumes it and may add engine-specific adapters only through its conformance rules. No program creates a second process runner.

`unknown` means no canonical identity was derived and is a coverage diagnostic. `recognized-but-unsupported` means identity is known but the requested capability/provider is not implemented. `unavailable` means a declared provider could not be used (for example missing, version-mismatched, timed out, or malformed). None is a clean mechanical pass.

## Parallel safety

Language packets may run in parallel only on disjoint grammar adapter, fixture, and proof paths. Canonical registries, Cargo workspace manifests, domain fact types, validator traits, scan dispatch, and capability matrices are singleton integration surfaces with one named owner at a time.
