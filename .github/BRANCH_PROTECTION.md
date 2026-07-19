# Branch protection contract

The `main` branch accepts changes only through a pull request whose stable
required check is green:

- `Rust CI / required`

The aggregate check mechanically requires graph-impact planning, all directly
affected Cargo packages and reverse workspace dependents, the complete
Windows/Linux/macOS workspace matrix, security/dependency/SBOM policy, and
frozen/native Enforcer dogfood. Implementation jobs remain independently
retryable, while the aggregate gives branch protection one stable context.

Repository protection must also:

- require the branch to be current with `main`;
- enforce checks for administrators;
- disallow direct and force pushes;
- disallow branch deletion;
- refuse red, skipped-required, cancelled, or pending checks.

The typed branch-protection verifier reads the live GitHub configuration and
fails closed on drift. If the workflow name or aggregate job id changes, the
typed desired protection and its fixtures must change in the same packet.

Release tags do not weaken this contract. The release workflow repeats the
frozen scan and format gate, executes fail/pass binary smoke fixtures for every
published target and variant, checksums the assets, and emits build-provenance
attestations before publishing.
