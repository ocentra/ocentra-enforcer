# cyberskills adapters (h12)

External, OPTIONAL tool-wrapper scripts for irreplaceable cyberskills
engines (symbolic execution / fuzzers / scanners / forensics — e.g.
slither, mythril, nmap, sqlmap, volatility, ghidra, boto3/azure-mgmt/
google-cloud SDK fetchers). None of the tools these scripts wrap are
required to be installed; the Rust seam in
`crates/enforcer-harness/src/adapters/cyberskills/` graceful-skips honestly
(never a silent pass) when a wrapped tool's binary/lib is absent.

This directory — and everything under it — is **excluded from the
enforcer's own self-scan** via the `ocentra-enforcer` profile's
`ignoreFileGlobs` (`crates/enforcer-harness/adapters/*`, coordinating with
h11's `vendor/*` entry for the vendored Python corpus). Scripts here may
be non-Rust (Python/shell) because the ENGINE they invoke is external —
the enforcer binary itself stays pure Rust; only this directory's wrapper
scripts are allowed to be a different language, and only because they are
never part of the enforcer's own dogfood scan.

No wrapper script is checked in yet. Per the h12 workpack charter ("build
this pack ONLY as the (d) engine-bound skills are actually needed — it is
the deferred, opt-in complement to h11"), this pack currently lands only:

- the graceful-skip seam (`src/adapters/cyberskills/seam.rs`)
- the RECORDED-output parsing boundary (`src/adapters/cyberskills/recorded.rs`)
- one representative thin severity gate (`src/adapters/cyberskills/gate.rs`,
  `CYBER-ADAPTER-SCA-SEVERITY.1`)
- this dogfood-exclusion seam (this directory + the profile glob entry)

A future pass adds a real wrapper script here (e.g. `slither.sh`) the day
an actual `d`-tier skill needs it; when it does, its Rust run-adapter goes
in `src/adapters/cyberskills/`, its output is parsed through
[`recorded::parse_recorded`](../../src/adapters/cyberskills/recorded.rs)
(or an equivalent live-subprocess variant of the same seam), and its gate
is registered in `crates/enforcer-rules/rules/` exactly like
`CYBER-ADAPTER-SCA-SEVERITY.1`.
