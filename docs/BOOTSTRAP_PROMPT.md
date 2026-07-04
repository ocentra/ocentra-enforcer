# Bootstrap Prompt For A Fresh Agent

<!-- ai-dense -->
```yaml
purpose: copy-paste prompt for a fresh AI-harness session to install the enforcer and wire a target repo
binary: single native `enforcer` executable; install downloads it and registers the MCP server, no toolchain
install_scope: user/global by default
placeholders: "<ENFORCER_INSTALL_PATH>, <TARGET_REPO_PATH>, <PROFILE> -- fill with real values, never leave a literal drive-letter example"
verify_step: "enforcer doctor --root <TARGET_REPO_PATH>; ask the harness to call mcp__enforcer__route"
```
<!-- /ai-dense -->

Use this when asking a fresh AI-harness session to install the enforcer and
wire a target repo. Fill in the bracketed placeholders with real values —
none of them should be left as a literal example path.

```text
You are setting up the enforcer.

Install:
1. Run the platform install script (or `cargo build --release --workspace` from
   source if no published binary exists for this platform), then:
   enforcer install --root <TARGET_REPO_PATH> --profile <PROFILE> --dry-run
2. If the plan is correct, run the non-dry-run installer:
   enforcer install --root <TARGET_REPO_PATH> --profile <PROFILE>
3. Verify the global MCP registry and target wiring:
   enforcer doctor --root <TARGET_REPO_PATH>
4. Restart the harness (or start a new session) so it reloads MCP servers.
5. Ask the harness to call mcp__enforcer__route for root <TARGET_REPO_PATH>,
   profile <PROFILE>, scope files, files ["Cargo.toml"] (or the target
   project's actual manifest file).
6. For hooks and CI, run the target adapter dry-run first:
   enforcer init --root <TARGET_REPO_PATH> --profile <PROFILE> --adapters precommit,github-actions --dry-run
7. Do not write hook or CI files until the dry-run plan is reviewed.

Rules:
- The MCP server is the installed enforcer binary, addressed by absolute path.
- The target repo is always passed as root.
- The installer updates harness config directly and creates a backup before writing.
- `enforcer doctor` verifies global config separately from MCP server smoke.
- Use `profile` for pack-owned policy.
- Use `configPath` for target-owned policy.
- Do not copy enforcer source into the target repo.
- Do not remove the target repo's existing guards until old-vs-new parity is proven.
- Use forward-slash paths in TOML/JSON on Windows to avoid backslash escaping issues.
```

## MCP Verification Prompt

After setup, ask the harness:

```text
Use the enforcer MCP server. Call mcp__enforcer__route with:
root = <TARGET_REPO_PATH>
profile = <PROFILE>
scope = files
files = ["Cargo.toml"]

Report the returned docs, rule IDs, and whether the route avoided loading the full rule corpus.
```

Expected result:

- `ok: true`.
- `docs` contains only matching rule docs/records.
- `rules` contains compact rule metadata, not the whole rule corpus.
