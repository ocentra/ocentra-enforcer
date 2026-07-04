# Security Policy

<!-- ai-dense -->
```yaml
report_to: enforcer maintainers (private)
disclosure: no PoC exploit details before a fix/mitigation ships
release_gate: "self-scan, dependency policy, secret scan, SBOM check, MCP smoke, rule coverage -- cargo build --workspace && cargo test --workspace clean"
supported: "current main branch + latest published release binary"
```
<!-- /ai-dense -->

Report vulnerabilities privately to the enforcer maintainers. Do not publish
proof-of-concept exploit details until a fix or mitigation is available.

Security fixes must include a regression test or an explicit written reason
why the behavior cannot be tested safely. Policy-critical changes must run
the enforcer self-scan, dependency policy, secret scan, SBOM check, MCP
smoke, and rule coverage checks before release.

Supported versions are the current `main` branch and the latest published
release binary.
