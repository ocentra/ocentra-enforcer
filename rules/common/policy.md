# Policy Integrity Rules

## Covered Rules

- `CFG-1.1`: Strict profiles must keep `error` in `failOn`. Empty or warning-only failure sets are bypass attempts.
- `CFG-1.2`: Immutable rules cannot be disabled.
- `CFG-1.3`: Immutable rules cannot be downgraded from `error` to `warning` or `info`.
- `CFG-1.4`: `allowUnsafeCode=true` requires a narrow governed waiver.
- `CFG-1.5`: `publicReexportPolicy: "allow"` is forbidden in strict profiles.
- `CFG-1.6`: `allowBuildRs`, `allowGitDependencies`, and `allowPathDependencies` require narrow governed waivers.
- `CFG-1.7`: Boundary glob additions require an owner note.
- `CFG-1.8`: Any rule disable, including advisory disables, requires waiver metadata with an expiry.
- `CFG-1.9`: Unknown config keys are forbidden.
- `CFG-1.10`: Configs must declare `schemaVersion` and `profileName` so layer precedence is unambiguous.
- `CFG-1.11`: Profile names must be known and locked.
- `CFG-1.12`: Config changes that opt into self-check mode must record `policyIntegrityChecked=true`.
- `WAIVER-1.1`: Waivers must include `ruleId`, `waiverId`, `owner`, `issue`, `reason`, `scope`, `expires`, `remediation`, and `ciAllowed`.
- `WAIVER-1.2`: Waiver scope must be exact or narrow. Repo-wide and language-wide waivers are forbidden.
- `WAIVER-1.3`: Expired waivers fail.
- `WAIVER-1.4`: Immutable rules cannot be waived unless the registry explicitly marks them `waivable`.
- `WAIVER-1.5`: CI waiver behavior must be explicit.
- `WAIVER-1.6`: Waivers must remain visible in output.
- `WAIVER-1.7`: Active waiver count must stay within `maxActiveWaivers` when configured.
- `WAIVER-1.8`: Waiver expiry must stay within `maxWaiverDays` (90 days by default).
- `WAIVER-1.9`: Waiver owner must be an accountable human or team, not `ai`, `codex`, `agent`, or empty metadata.
- `WAIVER-1.10`: Waivers require a remediation plan.

## Fails

```json
{
  "failOn": [],
  "rules": {
    "RR-4.1": { "enabled": false },
    "RR-6.1": { "severity": "warning" }
  },
  "allowUnsafeCode": true,
  "waivers": [
    {
      "ruleId": "RR-4.1",
      "waiverId": "W-1",
      "owner": "codex",
      "issue": "none",
      "reason": "temporary",
      "scope": ["**/*"],
      "expires": "2020-01-01",
      "remediation": "",
      "ciAllowed": false
    }
  ]
}
```

## Passes

```json
{
  "failOn": ["error"],
  "rules": {
    "DOC-1.1": {
      "enabled": false,
      "waiverId": "WAIVER-DOCS-LOCAL-2026-0001",
      "owner": "maintainers",
      "issue": "https://github.com/ocentra/ocentra-enforcer/issues/1",
      "reason": "Documentation comments are advisory for this repo while hard rules mature.",
      "scope": ["src/**", "scripts/**", "mcp/**"],
      "expires": "2026-12-31",
      "remediation": "Promote public API doc requirements only after policy docs are complete.",
      "ciAllowed": true
    }
  }
}
```

## Fix Recipe

1. Remove disable/downgrade overrides for immutable rules.
2. Keep strict profiles failing on `error`.
3. Keep unsafe, build scripts, git dependencies, path dependencies, and public re-export allow mode disabled unless a waiver is narrow and current.
4. Add a visible waiver only for rules whose registry lock allows it.
5. Make waiver scope exact or narrow, set an expiry, and include a remediation plan.

## Validator

- scanner: `common/config-lockdown`, `common/waiver-policy`, and `common/policy-integrity`
- implemented in: `src/policy.mjs` and `src/checks.mjs`
- command: `ocentra-enforcer check policy-integrity --root <repo>`
