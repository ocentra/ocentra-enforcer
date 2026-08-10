# Cyber Plan Engineering Graph

The executable graph is the control plane for the CyberSkills plan. It is not
a replacement for the Markdown plan and it is not a generic graph of every
Enforcer engineering task.

## Sources and authority

- `docs/engineering-graph.json` is the checked-in graph manifest.
- `docs/plans/cyberskills-parity-plan/README.md`, `WORKPACK_INDEX.md`, the
  workpack Markdown files, and `TEST_PROOF_EXPECTATIONS.md` remain the detailed
  intent and acceptance sources.
- `crates/enforcer-rules/dispositions/cyberskills-disposition.json` supplies
  catalog availability and separate implementation/proof coverage facts.
- `proof/cyberskills/cp08/**` supplies immutable decomposition evidence.
- The loader never reads `vendor/**`; the protected
  `detecting-fileless-malware-techniques` source remains an explicit
  `sourceUnavailable` catalog identity with no materialized path.

The graph does not promote a CP08 decomposition to native implementation or
executable proof. Those are independent coverage dimensions and remain
`none`, `partial`, or `complete` only when their own evidence says so.

## Model

The graph uses stable IDs and typed nodes:

| Node | Meaning |
| --- | --- |
| `Goal` | Cybersecurity plan outcome |
| `Plan` | CyberSkills parity plan |
| `Workpack` | CP00 through CP13 execution unit |
| `Skill` | One vendor catalog identity, not an implementation claim |
| `Requirement` | Imported workpack checklist item |
| `Test` | Named gate from the proof table |
| `Proof` | Committed evidence artifact, including CP08 batches |
| `Adr` | Reserved for linked architecture decisions |
| `Dependency` | A dependency owned by another plan or authority |

`DependsOn` edges are hard gates. Cycles and missing endpoints are validation
errors. `Ready` and `Blocked` are derived; agents must not edit them into the
manifest. `Done` is accepted only after its completion contract verifies paths,
evidence, and checklist completion.

## CLI

Run from the repository root:

```text
enforcer graph validate
enforcer graph status
enforcer graph ready
enforcer graph blocked
enforcer graph inspect WP/CP00
enforcer graph why WP/CP13
```

Each command emits JSON suitable for another tool or a human review. A failed
validation or an invalid graph query uses the normal Enforcer non-success exit
class. The graph is read-only in this first packet; lifecycle changes must
continue through the existing coordination/approval workflow until a future
mutation packet adds a guarded state store.

## Migration and uncertainty

The importer derives workpack IDs from filenames and dependencies from the
existing `deps:` lines. It preserves routing status as metadata and never
trusts a Markdown status cell as completion. Named test/proof rows become
completion-contract references; a `PENDING` row therefore cannot silently
become `Done`. Unresolved external dependencies remain explicit blocked graph
nodes, while malformed or missing endpoints fail `graph validate`.

This is the first executable Cyber Plan control-plane slice. It proves graph
loading, identity, dependency ordering, blocking, completion-contract
calculation, catalog accounting, and CP08 evidence linkage. It does not prove
that the CyberSkills rules, external engines, native implementations, or live
security outcomes are complete.
