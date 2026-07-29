---
name: enforcer-onboarding
description: Onboard a project onto the current native Rust Enforcer without assuming unreleased installer, MCP, proof, or CI capabilities.
---

# Enforcer Onboarding

Use this skill once for a new target repository. Current onboarding is a
verified binary install plus explicit CLI validation; automatic target-repo
scaffolding and CI wiring are not public native commands yet.

## 1. Inspect

Read the target's real manifests, languages, workspace structure, existing
policy, and existing CI. Do not overwrite or retire existing guards.

## 2. Verify The Binary

```powershell
enforcer --help
enforcer serve --help
```

Do not assume a release exists merely because installer scripts or automation
files are checked into the source repository.

## 3. Register Harnesses

```powershell
enforcer install
```

The command accepts no flags. It applies user-level registrations for all
supported adapters and runs its internal read-only health check. Restart the
harness afterward.

## 4. Verify MCP

Call `ocentra_enforcer_mcp_status`. The current Rust router additionally wires
coordination status, exact-path coordination claim, and UI launch/status.
Route, scan, check, diagnostics, proof, and broader coordination contracts are
registered but not wired.

## 5. Verify Target Validation

From the target repository, run a focused native CLI scope. Confirm a known
violation fails, repair it, and confirm the same scope passes:

```powershell
enforcer scan path/to/file
enforcer verify --mode local --all
```

Use paths, `--base`/`--head`, or `--all`; do not pass Node-compatibility
`--root`, `--profile`, `--files`, or `--workspace` flags to the native Rust
CLI.

## 6. Report

Report the verified binary, installed adapters, MCP status result, seeded-fail
result, clean-pass result, and any still-manual repository or CI wiring. Do not
report onboarding complete from file presence alone.
