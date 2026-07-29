# Bootstrap Prompt For A Fresh Agent

<!-- ai-dense -->
```yaml
purpose: copy-paste prompt for a fresh harness session using a verified native Rust build
install: "enforcer install (no flags; user-level adapter registration plus internal health check)"
mcp_smoke: "ocentra_enforcer_mcp_status"
validation: "run native check/scan/verify with paths, --base/--head, or --all"
```
<!-- /ai-dense -->

Use this only after building or otherwise obtaining a native binary whose
origin and `--help` output have been verified.

```text
You are setting up the native Rust Enforcer.

1. Confirm the binary contract:
   enforcer --help
   enforcer serve --help
2. Register its user-level harness adapters:
   enforcer install
3. Restart the harness so it reloads MCP configuration.
4. Call ocentra_enforcer_mcp_status and confirm the server name is enforcer.
5. From the target repository, validate one explicit scope:
   enforcer scan Cargo.toml
6. Widen only after the focused scope works:
   enforcer verify --mode local --all

Current boundary:
- install accepts no flags and performs its health check internally.
- Rust MCP currently wires server status, coordination status, exact-path
  coordination claim, and UI launch/status.
- plan, proof, and coordination are visible CLI groups but are not wired.
- route, scan, check, diagnostics, proof, and broader coordination MCP tools
  may be registered but return a not-wired response.
- do not invent release URLs, edit harness configuration by hand, or remove a
  target repository's existing guards without separately verified parity.
```
