# h11 Cyberskills Corpus To Rust Rules

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Cyberskills Corpus To Rust Rules`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `rules/security/cyberskills/**, src/validators/cyberskills-*.ts, src/validators/skill-frontmatter-lint.ts, src/threat-vocab/cyberskills-vocab.ts, tests/fixtures/cyberskills/**, tests/cyberskills/**`
- deps: `d01, h03, f05`
- tier: `P1/P2`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [vendor analysis](../../../../vendor/anthropic-cybersecurity-skills/RUST_CONVERSION_ANALYSIS.md).

## Where We Are
The `anthropic-cybersecurity-skills` corpus (817 skills, Apache-2.0, by mukul975) is vendored at `vendor/anthropic-cybersecurity-skills/` for preservation, but nothing MECHANIZES it. Its detection knowledge lives only as Python `agent.py` scripts (~273/282 sampled shell out to an external CLI) plus prose `SKILL.md`/`references/*.md`. None of it is expressed as our T1/T2 rules, none of its MITRE/NIST frontmatter feeds `h03`'s threat vocabulary, and none of it is wired into `f05`'s security-audit scope. The corpus is ALSO at risk of polluting our own dogfood: `ocentra-enforcer.config.json` `ignoreFileGlobs` does not yet list `vendor/**`, so the vendored Python would be scanned as if it were our code — exactly the "dogfood must not drown in vendored Python" failure the owner named. The full disposition (Rust-convertible ~55-65% of skill cores vs python-bound ~15-20%, the T1/T2/T3/adapter breakdown, harvest targets, and the mapping plan) is documented in `vendor/anthropic-cybersecurity-skills/RUST_CONVERSION_ANALYSIS.md`.

## Where We Want To Be
The FUNDAMENTAL-LOGIC skills (regex tables, boolean field predicates, HCL/YAML/JSON manifest parsing, entropy scans, Rego deny-rules) are REIMPLEMENTED as native Rust rules — scaffolded through `d01` so each lands in 5-way parity (ruleId <-> doc <-> validator <-> {fail+pass fixture} <-> detection test). Their MITRE ATT&CK + NIST-CSF frontmatter seeds `h03`'s canonical threat vocabulary, and the security-audit scope is wired through `f05`'s router so a detected repo routes to these packs and emits findings into the `g02` report with h03-validated threat citations. The genuinely python-lib-bound skills (irreplaceable engines: symbolic execution, fuzzing, network scan, binary/memory forensics) are NOT reimplemented here — they are deferred to the optional adapter pack `h12`, which is ignored from our dogfood. First pass is a THIN SLICE: the 7 highest-yield harvest targets + the frontmatter linter/vocab, not all 817 skills. GENERIC over the corpus; no single skill or engine is assumed present.

This pack does NOT redefine: the scaffolder/parity oracle (consumed from `d01`), the threat-vocabulary dictionary contract (consumed from `h03`), or the detect/route plan shape (consumed from `f05`). It ports RULE CONTENT and drops the CLI dependency for the (a)/(b) cluster.

## Requirement Checklist
Each rule is scaffolded via `enforcer rule new <ID>` (d01) with fail-fixture + pass-fixture + detection test. Rule content is harvested from the cited vendor paths (see RUST_CONVERSION_ANALYSIS.md harvest targets); CLI dependency is dropped in favor of a native Rust predicate over generic JSON/config/manifest input.

- [ ] **Dogfood exclusion (fail-closed).** `vendor/**` is added to `ocentra-enforcer.config.json` `ignoreFileGlobs`; a test asserts a scan of `vendor/anthropic-cybersecurity-skills/**` yields zero self-host findings (vendored Python never enters our dogfood).
- [ ] **T1 frontmatter lint** (`cyberskills.skill-frontmatter-valid`): port `tools/validate-skill.py` to Rust (8 required fields, kebab-case name <=64, description >=50, `domain==cybersecurity`, subdomain in the 46-entry allowlist, >=2 tags) AND EXTEND it to require `mitre_attack` ids match `T\d{4}(\.\d{3})?` and `nist_csf` ids match `(GV|ID|PR|DE|RS|RC)\.[A-Z]{2}(-\d{2})?` and be members of the h03 dictionary. Malformed/unknown id = fail (the gap the corpus validator leaves open).
- [ ] **T1 IaC/cloud/manifest cluster** (harvest targets 1, 5, 6, 7): scaffold at least `iac.tf.s3-encryption-required`, `iac.tf.iam-no-wildcard-action`, `iac.tf.sg-no-public-ssh-ingress` (from terraform Rego, SKILL.md L146-187); `cloud.azure.storage-public-blob`, `cloud.azure.storage-require-https`, `cloud.azure.storage-min-tls12` (azure predicates, agent.py L36-79); `web.headers.hsts-missing-or-weak`, `web.headers.csp-missing`, `web.cookie.secure-httponly-samesite` (headers, agent.py L46-80); `supplychain.dependency-confusion-claimable` (manifest parsers, agent.py L46-92). Each is a native Rust predicate over generic config/manifest input, no CLI.
- [ ] **T2 scored cluster** (harvest targets 2, 3): scaffold `detect.waf.sqli-signature` as a scored Rust matcher porting the 17-entry `SQLI_PATTERNS` regex+severity table and `MODSEC_RULE_MAP` 942xxx lexicon (`detecting-sql-injection-via-waf-logs/scripts/agent.py` L14-55). T2 = confidence/severity output over a labeled corpus, not single pass/fail.
- [ ] **h03 vocab seed** (`src/threat-vocab/cyberskills-vocab.ts`): a Rust frontmatter parser unions `mitre_attack` (261+ distinct ids) + `nist_csf` (~30 distinct ids) across all 817 SKILL.md, cross-checked against `mappings/attack-navigator-layer.json` (218 techniques; its skill lists are truncated so it is a CROSS-CHECK, not source of truth). Output is the canonical dictionary `h03` consumes. OWASP/CWE are NOT per-skill (0/817) — if needed they are derived from the `mappings/owasp/README.md` crosswalk tables, never assumed as frontmatter.
- [ ] **f05 -> g02 wiring:** the harvested packs register a `security-audit` scope that `f05`'s route-plan attaches for detected repos; findings carry `ruleId` + ATT&CK/NIST citation (validated by h03) + doc-anchor + file:line, in the shape `g02` renders. Honors f04 silent mode (consumed, not redefined).
- [ ] **Boundary honored:** only the (a)/(b) fundamental-logic cluster is reimplemented in Rust here; (d) irreplaceable-engine skills are NOT ported here — they are referenced out to `h12`. No CLI subprocess is introduced by this pack.
- [ ] Scaffolder output re-validates green under the d01 `rule-scaffold-parity` oracle over every `cyberskills.*`/`iac.*`/`cloud.*`/`web.*`/`supplychain.*`/`detect.*` id this pack adds.

## Acceptance And Proof
Tier P1 (T1 blocking rules) / P2 (T2 scored matchers). Per-rule fixtures under `tests/fixtures/cyberskills/<ruleId>/{fail,pass}.*` with minimal-repro pairs (e.g. an S3 block with vs. without `server_side_encryption_configuration`; a storage account `enable_https_traffic_only==false` vs `true`; a response missing HSTS vs `max-age>=31536000`; a package.json with a public-resolvable private-looking name vs a scoped/secure one). The T2 `detect.waf.sqli-signature` fixture is a labeled corpus (benign vs malicious WAF log lines) asserted on confidence+severity, not single pass/fail.

Representative triples:
- terraform s3 encryption: fail `tests/fixtures/cyberskills/iac.tf.s3-encryption-required/fail.tf` (no SSE block -> flagged), pass `.../pass.tf` (SSE configured -> clean), test `cyberskills-iac-tf.test`.
- azure require-https: fail `.../cloud.azure.storage-require-https/fail.json` (`enable_https_traffic_only:false`), pass `.../pass.json` (`true`), test `cyberskills-cloud-azure.test`.
- waf sqli score (T2): fail `.../detect.waf.sqli-signature/fail.log` (UNION SELECT line -> critical hit), pass `.../pass.log` (benign query -> below threshold), test `cyberskills-waf-sqli.test`.
- dogfood exclusion: fail-fixture `cyberskills-vendor-not-dogfooded` asserts a self-host scan of `vendor/**` returns zero findings (vendored Python ignored); a regression that scans it fails the test.
- frontmatter lint: fail `.../skill-frontmatter/fail_bad_attack_id.md` (malformed `T99` / unknown id -> flagged), pass `.../pass_valid.md`, test `cyberskills-frontmatter-lint.test`.

Detection tests iterate every fixture pair through the engine (fail-flagged / pass-clean or scored) and run the d01 parity oracle over the family. Named proof rows in TEST_PROOF_EXPECTATIONS.md: `cyberskills-rule-detection`, `cyberskills-frontmatter-and-vocab`, and `cyberskills-vendor-dogfood-exclusion`. Record detection-test artifact paths there.

## Parallel Ownership Notes
`owns:` is disjoint: `rules/security/cyberskills/**`, `src/validators/cyberskills-*.ts`, `src/validators/skill-frontmatter-lint.ts`, `src/threat-vocab/cyberskills-vocab.ts`, `tests/fixtures/cyberskills/**`, `tests/cyberskills/**` are new paths. The single edit outside that tree is adding `vendor/**` to `ocentra-enforcer.config.json` `ignoreFileGlobs` (one array entry, additive, no existing entry touched). Depends on `d01` (scaffolder + parity oracle — consumed, not redefined), `h03` (threat-vocabulary dictionary contract — this pack SEEDS the dictionary the h03 validator enforces; coordinate the vocab shape with h03, do not redefine its enforcement), and `f05` (detect/route plan shape — this pack REGISTERS a security-audit scope, does not reimplement the router). Distinct from `h12`: h11 reimplements the fundamental-logic (a)/(b) skills in native Rust and introduces NO subprocess; h12 owns the optional python/CLI run-adapters for the (d) irreplaceable-engine skills. Distinct from `f05`: f05 owns the router; h11 owns only the security-audit rule packs the router routes to. Must not touch existing `rules/rules.json` routing beyond the rows d01's scaffolder writes for these ids.
