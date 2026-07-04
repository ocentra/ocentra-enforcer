# Rust Conversion Analysis — Anthropic-Cybersecurity-Skills corpus

Scope: assess whether the vendored `anthropic-cybersecurity-skills` corpus (817 skills, Apache-2.0,
`skills/<name>/{SKILL.md, scripts/agent.py, scripts/process.py, references/*.md}` + `mappings/` +
`tools/validate-skill.py`) can be mechanized into OUR enforcer format — deterministic T1 validators with
fail/pass fixtures + detection tests, scored T2 matchers, T3 labeled-prose how-tos, and optional
python/CLI run-adapters — without drowning our own dogfood scan in vendored Python.

This is a **source-for-mechanization** analysis, not a plan to run the corpus as-is. The corpus stays in
`vendor/` for preservation and is EXCLUDED from the enforcer's own dogfood scan (see the sibling
`README.md` and the `ignoreFileGlobs` requirement below).

---

## Verdict

**Doable on a first pass — YES, incrementally.** The corpus is far more mechanizable than its Python
surface suggests. The vendored `agent.py` files are overwhelmingly thin `subprocess + argparse + json`
glue (~150–220 lines each) that shell out to an external CLI and re-parse its JSON; ~273/282 sampled
`agent.py` files import `subprocess`. But the RULE-ABILITY question is about the CORE IDEA each skill
encodes, and that core is frequently a small, language-agnostic predicate: a regex table, a boolean
field check on a config/manifest, an entropy scan, an HCL/YAML/JSON structural assertion. That is
exactly Rust's wheelhouse and satisfies the owner's "dogfood must not drown in vendored Python" goal.

The first pass is a THIN SLICE, not a boil-the-ocean port: harvest the ~7 highest-yield rule-content
assets (below), scaffold them through `d01`, wire the frontmatter vocabulary into `h03`, and route the
security-audit scan (`f05`) into the `g02` report. Everything else (the T3 how-to prose and the
irreplaceable-engine adapters) is ingest-only or deferred to an optional adapter pack.

- **Rust-convertible (fundamental logic):** ~55–65% of skill CORES.
- **Python/CLI-bound (keep as optional adapter, ignored from dogfood):** ~15–20% of skill CORES.
- **T3 prose (no code to convert — ingest as labeled how-to):** ~40–45% of skills.

(The convertible and prose buckets overlap: a T3 how-to can still contain an embedded T1/T2 sub-check
that we harvest. Percentages are of skill CORES, not files, and are extrapolated from ~22 skills read
in depth plus the full 817-name / verb-prefix / import distribution — high confidence on the harvest
targets, medium on the numeric split.)

---

## % Rust-convertible vs python-bound

| Bucket | Share of skill cores | Disposition |
|---|---|---|
| (a) T1 mechanizable rule | ~15–20% (~120–160 skills) | **Reimplement in Rust** — boolean field predicates, regex+entropy, HCL/YAML/JSON manifest parsing, Rego deny-logic. Drop the CLI dependency. |
| (b) T2 scored heuristic | ~20–25% (~165–205 skills) | **Reimplement in Rust** as scored matchers — the regex/heuristic tables are already inline data (e.g. `SQLI_PATTERNS`). |
| (c) T3 how-to procedure | ~40–45% (~330–370 skills) | **Ingest as labeled prose** — pentest/forensics/IR/threat-intel needing human+tool judgment. No validator, no conversion. |
| (d) Tool-invocation adapter | ~15–20% (~120–160 skills) | **Optional python/CLI run-adapter** — irreplaceable engine (symbolic exec, fuzzing, network scan, binary/memory forensics). Excluded from dogfood. |

**Convertible total = all of (a) + most of (b) ≈ 55–65% of cores.** The mapping/vocab/validator layer
converts with **zero Python residue**: all four corpus mapping surfaces are static data (Navigator
JSON, prose crosswalk tables, thin `index.json`) → pure ingestion; `tools/validate-skill.py` is stdlib
regex/string only → 1:1 Rust reimplement.

**Genuinely python/tool-bound (bucket d):** symbolic execution / formal analysis (mythril, slither,
foundry/forge), network scanners (nmap, nessus, openvas, nikto), fuzzers/exploit frameworks (sqlmap),
binary & memory forensics (volatility, ghidra, apktool, MobSF, chipsec, peepdf, autopsy), and cloud-SDK
live-inventory fetchers (boto3, azure-mgmt-*, google-cloud-*). **Note the SDK cases are only half-bound:**
the SDK fetch stays python, but the PREDICATE over the fetched state should be Rust and fed generic
JSON/config so it also runs against IaC/manifests offline. Emitted Sigma/YARA/SPL/KQL is rule *text*
for external engines, not a python-lib dependency — we generate it as data, we do not run a python lib.

---

## Rule-ability breakdown with concrete examples and proposed ruleIds

### (a) T1 mechanizable — deterministic pass/fail → `ruleId` + Rust validator + fail/pass fixtures + detection test

Each check has an obvious minimal-repro fixture pair (e.g. an S3 block with vs. without
`server_side_encryption_configuration`; a Dockerfile `USER root` vs `USER app`).

| Skill (source) | Proposed ruleId(s) |
|---|---|
| `auditing-terraform-infrastructure-for-security` (inline OPA/Rego, SKILL.md L146–187) | `iac.tf.s3-encryption-required`, `iac.tf.iam-no-wildcard-action`, `iac.tf.sg-no-public-ssh-ingress` |
| `detecting-azure-storage-account-misconfigurations` (boolean predicates, agent.py L36–79) | `cloud.azure.storage-public-blob`, `cloud.azure.storage-require-https`, `cloud.azure.storage-min-tls12` |
| `performing-security-headers-audit` (HSTS/CSP, agent.py L46–80) | `web.headers.hsts-missing-or-weak`, `web.headers.csp-missing`, `web.cookie.secure-httponly-samesite` |
| `implementing-secret-scanning-with-gitleaks` / `implementing-secrets-scanning-in-ci-cd` | `secret.hardcoded-credential` (regex + entropy over files/diff) |
| `detecting-dependency-confusion` (manifest parsers, agent.py L46–92) | `supplychain.dependency-confusion-claimable` |
| `analyzing-sbom-for-supply-chain-vulnerabilities` / `generating-and-analyzing-sboms` | `supplychain.sbom-missing`, `supplychain.sbom-cve-threshold` |
| `scanning-kubernetes-manifests-with-kubesec` / `implementing-kubernetes-pod-security-standards` | `k8s.pod.privileged-container`, `k8s.pod.host-network`, `k8s.pod.readonly-rootfs`, `k8s.pod.no-run-as-root` |
| `auditing-kubernetes-rbac-privilege-escalation` | `k8s.rbac.wildcard-verb-or-resource`, `k8s.rbac.cluster-admin-binding` |
| `hardening-docker-daemon-configuration` / `hardening-docker-containers-for-production` | `container.docker.no-privileged`, `container.docker.no-root-user`, `container.dockerfile.no-latest-tag` |
| `auditing-aws-s3-bucket-permissions` / `remediating-s3-bucket-misconfiguration` | `cloud.aws.s3-public-acl`, `cloud.aws.s3-encryption-required` |
| `securing-aws-iam-permissions` / `auditing-gcp-iam-permissions` | `cloud.iam.overpermissive-wildcard` |
| `testing-cors-misconfiguration` | `web.cors.wildcard-with-credentials` |
| `testing-jwt-token-security` / `performing-jwt-none-algorithm-attack` (detect half) | `auth.jwt.alg-none-accepted`, `auth.jwt.weak-secret` |
| `configuring-tls-1-3` (audit half) | `net.tls.legacy-version-enabled` (TLS1.0/1.1 = fail) |

### (b) T2 scored — pattern + confidence/severity, no crisp single-value pass/fail → scored Rust matcher + labeled corpus

| Skill (source) | Proposed ruleId |
|---|---|
| `detecting-sql-injection-via-waf-logs` (17-entry `SQLI_PATTERNS` + `MODSEC_RULE_MAP`, agent.py L14–55) | `detect.waf.sqli-signature` |
| `analyzing-email-headers-for-phishing-investigation` (mostly T2; SPF/DKIM/DMARC sub-checks are T1) | `detect.email.phishing-header-anomaly` (+ `auth.email.spf-dkim-dmarc-fail` T1) |
| `detecting-t1055-process-injection-with-sysmon` / `hunting-for-process-injection-techniques` | `detect.edr.process-injection` |
| `detecting-suspicious-oauth-application-consent` / `detecting-oauth-token-theft` | `detect.oauth.suspicious-consent` |
| `analyzing-dns-logs-for-exfiltration` | `detect.dns.exfil-entropy` |
| `analyzing-powershell-script-block-logging` | `detect.powershell.obfuscation-score` |
| `detecting-s3-data-exfiltration-attempts` / `analyzing-cloud-storage-access-patterns` | `detect.cloud.exfil-anomaly` |
| `auditing-mcp-servers-for-tool-poisoning` / `detecting-indirect-prompt-injection` | `detect.ai.prompt-injection-signature` |

### (c) T3 how-to — procedure needing human/tool judgment → labeled-prose skill, no validator

Examples: `exploiting-sql-injection-vulnerabilities`, `exploiting-idor-vulnerabilities`,
`exploiting-insecure-deserialization`, `performing-blind-ssrf-exploitation`,
`performing-kubernetes-penetration-testing`, `exploiting-zerologon-vulnerability-cve-2020-1472`,
`analyzing-memory-dumps-with-volatility`, `analyzing-linux-kernel-rootkits`,
`analyzing-golang-malware-with-ghidra`, `acquiring-disk-image-with-dd-and-dcfldd`,
`implementing-diamond-model-analysis`, `analyzing-cyber-kill-chain`,
`achieving-cmmc-level-2-compliance`, `performing-soc2-type2-audit-preparation`,
`collecting-open-source-intelligence`, `monitoring-darkweb-sources`. These are inherently
interactive/judgment-driven (authorized-pentest gating, manual triage, evidence handling, compliance
narrative). They map to our T3 labeled-prose layer via the SKILL.md `Steps` + `Validation Criteria` +
`references/` — with an optional detection-test on any embedded (a)/(b) sub-check they mention.

### (d) Tool-invocation — skill core IS "drive external CLI X" → optional run-adapter, kept OUT of Rust dogfood

Examples: `analyzing-ethereum-smart-contract-vulnerabilities` (slither + mythril symbolic exec),
`auditing-foundry-smart-contract-security` (forge), `scanning-network-with-nmap-advanced` (nmap),
`performing-vulnerability-scanning-with-nessus`, `exploiting-sql-injection-with-sqlmap` (sqlmap),
`performing-authenticated-scan-with-openvas`, `benchmarking-kubernetes-with-kube-bench`,
`analyzing-android-malware-with-apktool`, `performing-android-app-static-analysis-with-mobsf`,
`auditing-uefi-firmware-with-chipsec`, `analyzing-malicious-pdf-with-peepdf`. For these the RESULT
(CVE list, CIS findings, benchmark score) can feed a thin T1/T2 gate wrapper (adapter emits findings →
gate on severity), but the analysis engine itself stays external.

### Boundary insight (drives the whole plan)

Many skills wear two hats. `scanning-kubernetes-manifests-with-kubesec`'s vendored script is (d)
tool-invocation, but the underlying checks (privileged / hostNetwork / runAsNonRoot /
readOnlyRootFilesystem) are (a) mechanizable rules we should REIMPLEMENT in Rust as native manifest
validators — kubesec is just one way to compute them. Same for terraform (checkov CLI vs. inline Rego
→ Rust HCL validators), trivy IaC (external vs. native), gitleaks (external CLI but the secret
regex+entropy is trivially native Rust). **RECOMMENDATION: for the (a) cluster, port the RULE CONTENT
(regex tables, boolean field predicates, Rego deny-rules) to native Rust validators and DROP the CLI
dependency; fall back to a run-adapter only where the engine is irreplaceable.**

---

## Highest-yield harvest targets (copy-paste-portable rule content already in the corpus)

Fastest path to seeding our T1/T2 ruleset — these are inline data, not deep algorithms:

1. Inline Rego deny-rules in `auditing-terraform-infrastructure-for-security/SKILL.md` (L146–187): S3
   encryption, IAM wildcard, SG public ingress → Rust HCL validators.
2. `SQLI_PATTERNS` 17-regex+severity table in
   `detecting-sql-injection-via-waf-logs/scripts/agent.py` (L14–32).
3. `MODSEC_RULE_MAP` 942xxx SQLi rule-id lexicon (same file, L34–55).
4. `SWC_REGISTRY` smart-contract weakness taxonomy in
   `analyzing-ethereum-smart-contract-vulnerabilities/scripts/agent.py` (L14–25) — taxonomy is portable
   even though the slither/mythril engine is not.
5. Boolean field predicates in `detecting-azure-storage-account-misconfigurations/scripts/agent.py`
   (L36–79) and analogous `auditing-aws-s3` / `auditing-gcp-iam` scripts.
6. HSTS/CSP/cookie predicates in `performing-security-headers-audit/scripts/agent.py` (L46–80).
7. Manifest-name parsers + registry-404 logic in `detecting-dependency-confusion/scripts/agent.py`
   (L46–92).

Also a T1 goldmine: `references/api-reference.md` (present in 810/817 skills) carries machine-checkable
detection content — e.g. `detecting-credential-dumping` has `GrantedAccess` bitmask tables
(`0x1010`=Mimikatz, `0x1FFFFF`=PROCESS_ALL_ACCESS), literal command-line strings
(`reg save hklm\sam`; `rundll32 comsvcs.dll MiniDump`; `ntdsutil`), and ready-to-run Splunk SPL / Sigma
queries. These regex/string/config detection patterns are exactly what our T1 validators encode; the
paired `agent.py` pattern-constant lists are fixture seeds.

**Fixture strategy:** (a) T1 skills map cleanly to our fail/pass fixture doctrine (each check has a
minimal-repro pair). (b) T2 skills need labeled corpora (benign vs malicious log lines) rather than
single fixtures. (c) T3 skills have no validator → prose + optional detection-test on embedded sub-checks.

---

## MITRE / OWASP mapping plan for h03 (threat vocabulary)

**Directly usable, machine-readable, no scraping.** Every SKILL.md carries a top-level `mitre_attack:`
YAML list (817/817 = 100%) and `nist_csf:` list (816/817 = 99%). IDs are canonical and well-formed:

- ATT&CK matches `T\d{4}(\.\d{3})?` — e.g. `T1190` in 229 skills, `T1078` in 212, `T1059` in 134.
- NIST-CSF matches `(GV|ID|PR|DE|RS|RC)\.[A-Z]{2}(-\d{2})?` — e.g. `DE.CM-01` in 484, `DE.AE-02` in
  270, `ID.RA-01` in 263, `PR.IR-01` in 217.

Beyond those: 139/817 skills carry `d3fend_techniques:` (defensive countermeasures) and 94/817 carry a
nested `mitre_f3:` block (F3EAD threat-hunting, F-prefixed ids like `F1006.002`).

**h03 vocabulary plan:**
1. **Build the canonical dictionary** with a Rust frontmatter parser: union `mitre_attack` (261+ distinct
   ids) + `nist_csf` (~30 distinct ids) across all 817 SKILL.md blocks. Cross-check against
   `mappings/attack-navigator-layer.json` (a real MITRE Navigator v4.5 layer, ATT&CK v14, 218 technique
   entries as a technique→skills reverse index — use as a CROSS-CHECK, not source of truth, because its
   skill lists are truncated to ~5 names + "(+N more)").
2. **Adopt three axes:** ATT&CK = required primary threat vocabulary; NIST-CSF = control/compliance axis;
   D3FEND = optional defense-mapping axis. All three are already normalized in-frontmatter.
3. **Enforce well-formedness + membership:** require every f05 finding / g02 threat to cite an id from
   the dictionary; reject malformed or unknown technique ids. This is precisely the enforcement point
   the corpus's own validator omits (see next section).

**CRITICAL GAP — OWASP/CWE are NOT per-skill.** 0/817 SKILL.md carry `owasp:`, `cwe:`, `capec:`, or
`d3fend:` as top-level keys. OWASP Top-10 (2025) exists ONLY as prose in `mappings/owasp/README.md`,
mapped at SUBDOMAIN granularity (e.g. "web-application-security (41 skills) → A03 Injection"), NOT
per-skill. So OWASP/CWE cannot be REQUIRED as a per-threat citation without us first BUILDING that
mapping. The owasp README does provide two hand-authored crosswalk tables to seed from:
OWASP→ATT&CK (A03→T1190/T1059, A01→T1078/T1548, ...) and OWASP→NIST-CSF (A03→PR.DS/DE.AE, ...). If h03
wants OWASP citations, derive them from those crosswalk tables — do not assume per-skill OWASP tags exist.

---

## Corpus validator → direct T1 analog

`tools/validate-skill.py` is a pure-stdlib (regex + string) frontmatter linter and a ready-made
Rust-conversion exemplar. It enforces 8 REQUIRED_FIELDS (name/description/domain/subdomain/tags/version/
author/license), kebab-case name (`^[a-z0-9]+(-[a-z0-9]+)*$`), name ≤ 64 chars, description ≥ 50 chars,
`domain == cybersecurity`, subdomain in a 46-entry allowlist (with alias→canonical normalization), and
≥ 2 tags. Zero non-stdlib deps → trivially reimplement in Rust as a `d01`-scaffolded structural gate.

**It does NOT validate `mitre_attack`/`nist_csf` id FORMAT or membership** — that is the gap our Rust
validator should close (require well-formed + known-vocabulary technique ids), and that becomes h03's
enforcement hook.

---

## Wiring: f05 security-audit → g02 report (+ d01, h03)

1. **`f05` security-audit domain (scan → route):** the `api-reference.md` detection patterns + `agent.py`
   pattern-constant logic become f05 scan rules. `agent.py`'s shape (regex/pattern constant lists +
   stdlib parse → JSON alert objects) is exactly what f05 emits. f05's router routes the security-audit
   scope to the harvested cyber rule packs (a/b) and, for (d) skills only, to the optional python/CLI
   run-adapters via f03's native-tie config (run native AND ours, graceful-skip when the binary is absent).
2. **`g02` security report:** each SKILL.md already declares `Expected Output: JSON report ... with MITRE
   ATT&CK mapping`; the frontmatter `mitre_attack`/`nist_csf` ids are the report's classification /
   traceability fields. g02 renders the rule-by-rule violation matrix with each row carrying `ruleId`,
   the ATT&CK/NIST citation (validated by h03), the forbidden-behavior text, the WHY/doc-anchor, and
   file:line. Honors f04 silent mode (no UI during inline agent runs).
3. **`d01` mechanization engine:** the 8-field frontmatter contract + `validate-skill.py` + the fixed
   SKILL.md/references template are a concrete scaffold spec — one T1 validator per detection table,
   fixtures seeded from the `agent.py` pattern lists, all landing in 5-way parity (ruleId ↔ doc ↔
   validator ↔ {fail+pass fixture} ↔ detection test).
4. **`h03` threat vocab:** seed the canonical dictionary from the union of frontmatter ids, cross-checked
   against the Navigator layer; require every f05 finding / g02 threat to cite a dictionary id.

Data flow: `SKILL.md frontmatter + api-reference.md tables` → (d01 scaffold) → `Rust ruleId + validator
+ fixtures` → (f05 detect/route, security-audit scope) → `.enforce/ scan output` → (g02) → `violation
matrix with h03-validated ATT&CK/NIST citations`.

---

## Attribution obligations (Apache-2.0)

`LICENSE` is standard Apache-2.0 (top-level + a per-skill copy in 816/817 skill dirs). There is **no
NOTICE file**, so none to reproduce. **Naming trap:** the repo is titled "Anthropic-Cybersecurity-Skills"
but is NOT authored/owned by Anthropic — `CITATION.cff` and `README.md` name the author as **Mahipal**
(github `mukul975`, `mukuljangra5@gmail.com`), version 1.1.0, released 2026-03-21, repo
`github.com/mukul975/Anthropic-Cybersecurity-Skills`; per-skill `author:` frontmatter is
`mahipal`/`mukul975`. To comply, we: keep the `LICENSE` file(s) in `vendor/`, preserve the
`author`/`license` frontmatter, retain `CITATION.cff`, state that our conversion is a modified Derivative
Work, attribute the upstream author, and do NOT imply Anthropic authorship despite the misleading repo
name. See the sibling `README.md` for the attribution text we ship.

---

## First-pass recommendation (thin slice)

1. Add `vendor/**` to `ocentra-enforcer.config.json` `ignoreFileGlobs` (currently absent) so the vendored
   Python never enters our dogfood scan. Rust already ignores it (`rust-rules.config.json` `rustRoots`
   are `src`/`crates`/`tools`).
2. Port `tools/validate-skill.py` → Rust as a `d01`-scaffolded structural gate, EXTENDED to enforce
   ATT&CK/NIST id well-formedness + dictionary membership (the h03 hook).
3. Harvest the 7 rule-content assets above → `d01`-scaffolded T1/T2 rules with fixtures.
4. Build the h03 frontmatter vocabulary extractor + validator, cross-checked against the Navigator layer.
5. Wire the security-audit scope into f05 → g02, with (d)-bucket skills behind optional graceful-skip
   python/CLI adapters (the h12 pack).

Non-convertible remainder stays as-is: (c) T3 prose is ingested as labeled how-to (no conversion), and
(d) irreplaceable engines live in the optional adapter pack, both excluded from the enforcer's own
dogfood.
