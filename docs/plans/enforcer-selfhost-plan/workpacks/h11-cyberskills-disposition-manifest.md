# H11 CyberSkills disposition manifest

`crates/enforcer-rules/dispositions/cyberskills-disposition.json` is the retention
ledger for all 817 vendor catalog entries. It is deliberately separate from
the native-rule catalog: a native `CYBER-*` rule is not evidence that it
implements a particular vendor skill merely because the subjects look alike.

The `rules/` directory is reserved exclusively for JSON arrays that decode as
native `WireRuleRecord` catalogs. The disposition is a JSON object, so it lives
under `dispositions/`; the `cyberskills_disposition` integration test verifies
both contracts mechanically.

The manifest currently records:

| Disposition | Count | Meaning |
|---|---:|---|
| native | 0 | No per-skill vendor-to-native mapping has verified evidence yet. |
| unported | 282 | T1/T2 catalog candidates without an evidenced native mapping. |
| adapter-deferred | 399 | Requires a named external engine or live system; no per-skill adapter record is registered. |
| advisory-prose | 136 | Procedural or advisory material rather than directly executable policy. |

The integration test `cyberskills_disposition` rejects missing or duplicate
catalog paths, invalid dispositions, count drift, an absent evidence file, or
a `nativeRuleId`/`adapterRuleId` that does not resolve in the appropriate Rust
registry. It also intentionally proves the current mapped count is zero.

This is a retention and honest-disposition checkpoint, not authorization to
remove vendor content. The vendor corpus remains required until every retained
entry has an explicit disposition approved for removal and the pre-removal
dogfood evidence is refreshed.
