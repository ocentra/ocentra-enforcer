# Proof Contract Rules

## Covered Rules

- `PROOF-1.1`: PR-ready claims require fresh proof.
- `PROOF-1.2`: Proof freshness is tied to commit hash and file scope.
- `PROOF-1.3`: Manual-required proof cannot pass automatically.
- `PROOF-1.4`: Required artifacts must exist and be non-empty.
- `PROOF-1.5`: Required artifacts must hash-match recorded proof.
- `PROOF-1.6`: Dirty worktrees invalidate PR-ready claims unless explicitly allowed.
- `PROOF-1.7`: Waived or unavailable proof must remain visible.
- `PROOF-1.8`: Proof commands cannot be empty unless the proof is manual/unavailable.
- `PROOF-1.9`: Proof commands must be argv arrays, not shell strings.
- `PROOF-1.10`: Proof registry docs paths must exist.
- `PROOF-1.11`: Proof capabilities must match the runner environment.
- `PROOF-1.12`: Android/iOS/device proofs cannot auto-pass on a desktop-only runner.
- `PROOF-1.13`: Proof claims must list proved and unproved claims.
- `PROOF-1.14`: Proof output is compact by default.
- `PROOF-1.15`: Proof exports and artifacts must redact secrets.

## Fails

- A PR-ready claim references no proof run.
- A proof run has no command and is still marked passed.
- Manual/device evidence is absent but the proof passes.
- An artifact hash differs from the recorded proof run.
- A dirty worktree is claimed PR-ready without `allowDirty`.
- Proof artifact text leaks tokens or private keys.

## Passes

- Proof runs write `.enforce/proofs/runs/<runId>/proof-run.json`, diagnostics, events, summary, and artifacts.
- Claims verify commit, dirty state, scope, capability, required artifact hashes, and explicit proved/unproved claim lists.
- Manual/device proofs are `manual-required`, `unavailable`, `waived`, or passed with explicit evidence.
- Export is a compact manifest by default; raw artifacts require explicit artifact queries.

## Fix Recipe

1. Route proof selection through `proof/INDEX.md` and `proof/proofs.json`.
2. Run proof through `ocentra-enforcer proof run` or import legacy artifacts before claiming.
3. Keep proof outputs local under `.enforce/proofs`; CI should recollect and upload short-retention artifacts.
4. Use `proof claim --pr-ready` only after matching commit/scope/profile evidence exists.
5. Redact secrets before returning artifact text or export manifests.

## Validator

- scanner: `common/proof-contracts`
- implemented in: `src/proof.mjs`
- command: `ocentra-enforcer check proof-contracts --root <repo>`
