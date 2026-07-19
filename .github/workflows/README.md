# Enforcer CI contract

The repository uses one mechanically verified gate graph:

1. `ci.yml` computes changed Cargo packages and every reverse workspace
   dependent from `cargo metadata`.
2. The affected graph is validated first so failures return quickly.
3. Non-documentation changes then run the complete Windows, Linux, and macOS
   workspace gate, the exact `npm run ci:local` developer command,
   security/dependency/SBOM policy, and frozen/native dogfood.
4. `Rust CI / required` is the stable branch-protection context. Its aggregate
   job fails unless every relevant upstream job succeeds.
5. Version tags enter `release.yml` only after another frozen/native dogfood,
   format, dependency, advisory, and SBOM gate. Each target runs on a matching
   CPU/OS runner, and every target/variant executes seeded fail/pass binary
   smoke tests before packaging. Published assets carry checksums, SBOM
   evidence, and GitHub build-provenance attestations.

The checked-in workflows are product behavior. Migration notes, temporary
baselines, and local machine paths are not part of the generated consumer
contract. `node scripts/ci/verify-workflow-contract.mjs` rejects a missing
required gate or mutable major-version action reference.

Local equivalents:

```powershell
node scripts/ci/plan-impacted.mjs --base origin/main --head HEAD
node scripts/ci/run-impacted.mjs --packages enforcer-domain,enforcer-scan
node scripts/ci/verify-workflow-contract.mjs
node scripts/rust-rules.mjs scan --root . --languages rust --workspace
node scripts/check-cargo-workspace-members.mjs --fmt-check
```
