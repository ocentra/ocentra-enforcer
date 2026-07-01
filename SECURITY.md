# Security Policy

Report vulnerabilities privately to the Ocentra maintainers. Do not publish
proof-of-concept exploit details until a fix or mitigation is available.

Security fixes must include a regression test or an explicit written reason
why the behavior cannot be tested safely. Policy-critical changes must run the
Enforcer self-scan, dependency policy, secret scan, SBOM check, MCP tests, and
rule coverage checks before release.

Supported versions are the current `main` branch and the latest published
package version.
