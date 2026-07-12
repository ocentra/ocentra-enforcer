# h11 Wave 1 — Cyberskills Config-Scanner Rules

Follow-on to `h11-cyberskills-corpus-to-rust-rules.md`. h11 shipped the 7 named
harvest targets; Wave 1 extends the same native-Rust cyberskills family in
`crates/enforcer-lang-security/src/rules/cyberskills/**` with the highest-yield
**T1 deterministic config/manifest scanners** identified by the full-corpus
mechanization triage (`../refs/cyberskills-mechanization-catalog.md`). No CLI
subprocess is introduced; every rule is a native `Validator` over parsed
config/manifest input, proven by a fixture corpus.

## Requirement Checklist

- [x] **`CYBER-K8S-POD.1` — Kubernetes pod-security hardening**
  (`rules/cyberskills/k8s_pod_security.rs`). Harvested 1:1 from the vendored
  `scanning-kubernetes-manifests-with-kubesec`,
  `implementing-kubernetes-pod-security-standards`, and
  `implementing-pod-security-admission-controller` `agent.py` scripts. Parses a
  workload manifest (YAML or JSON) for `Pod` / `Deployment` / `DaemonSet` /
  `StatefulSet` / `ReplicaSet` / `Job` and emits one `Finding` per violated
  pod-security-standards (restricted) check: `privileged`, `hostNetwork`,
  `hostPID`, `hostIPC`, `allowPrivilegeEscalation` (must be explicit false),
  `runAsUser == 0`, `runAsNonRoot` (must be true), `readOnlyRootFilesystem`
  (must be true), added Linux capabilities (Critical for `ALL`/`SYS_ADMIN`),
  `drop: ["ALL"]` (must be present), and `hostPort`. Proven by a labeled
  YAML/JSON manifest corpus (`tests/cyberskills_corpus.rs`).

- [x] **`CYBER-DOCKER.1` — Dockerfile hardening**
  (`rules/cyberskills/dockerfile_hardening.rs`). Harvested from the vendored
  `hardening-docker-containers-for-production`,
  `performing-container-image-hardening`, and
  `implementing-container-image-minimal-base-with-distroless` skills. A line-scan
  (multistage-`AS`-alias aware, `\`-continuation aware) over Dockerfile text
  emitting a `Finding` per violated deterministic security check: unpinned /
  `:latest` base image, running as root (no `USER` or final `USER root`/`0`),
  `curl|wget ... | sh` remote-exec pipe, hardcoded secret in `ENV`/`ARG`, `ADD`
  of a remote URL, and `apt-get install` without `--no-install-recommends`.
  Proven by a 32-case labeled Dockerfile corpus. Heuristic / image-inspection
  checks (minimal-base choice, multistage split, setuid stripping,
  package-manager removal, HEALTHCHECK) are intentionally NOT emitted (they
  would over-flag) and are follow-ups.

- [x] **`CYBER-SECRET.1` — hardcoded provider credentials**
  (`rules/cyberskills/provider_credentials.rs`). Harvested from the inline
  secret-pattern tables in the vendored `implementing-api-key-security-controls`,
  `testing-for-sensitive-data-exposure`,
  `detecting-aws-credential-exposure-with-trufflehog`, and sibling skills. A
  gitleaks-style high-confidence provider ruleset over source lines: AWS
  `AKIA`/`ASIA` access keys + context-gated 40-char secret, GitHub
  `gh[pousr]_`, Stripe `sk_live_`/`rk_live_`, Google `AIza`, Slack `xox…`, npm
  `npm_`, PEM private-key blocks, and JWTs (Warning). Additive to `SEC-1.1`
  (which covers `key = "…"` assignments); catches the bare provider-key
  literals `SEC-1.1` misses. Deliberately EXCLUDES low-precision generic
  patterns (bare 40-char base64, 32-hex, bearer, email/SSN/PII) — FP magnets
  that would erode a prevention gate. Matched secrets are redacted in the
  finding snippet. Proven by a CODE-BUILT corpus in the module test
  (`provider_credential_corpus_code_built`): every provider-secret input is
  assembled from a prefix + filler at runtime so no real-secret-shaped
  literal is committed (GitHub push protection would correctly block that —
  the very leak this rule prevents). Flag cases per provider (incl.
  in-comment) + the FP cases a gate must not trip (env refs, MD5/git-sha
  hex, UUID, base64 without aws-context, email, short/placeholder keys). The
  on-disk oracle fixture uses AWS's allowlisted documented example key.

- [x] **`CYBER-AWS.1` — AWS Terraform resource hardening**
  (`rules/cyberskills/cloud_aws.rs`). Harvested from the vendored
  `auditing-aws-s3-bucket-permissions`,
  `remediating-s3-bucket-misconfiguration`, and `securing-aws-iam-permissions`
  skills; reuses `iac_terraform`'s HCL block parser. Per-resource checks over
  `.tf`: public S3 ACL (`acl = "public-read"`/`"public-read-write"`),
  `aws_db_instance.publicly_accessible = true`, `aws_ebs_volume` not
  `encrypted = true`, and `aws_security_group`/`_rule` ingress from
  `0.0.0.0/0` reaching a sensitive non-SSH port (RDP/MySQL/PostgreSQL/Redis/
  MongoDB/MSSQL/Elasticsearch/Telnet) or a wide-open range. Proven by a
  28-case HCL corpus (per-check hits + the FP cases: private ACL, restricted
  CIDR, egress, HTTP/HTTPS ports, SSH-only, absent attrs, non-AWS resources).
  Cross-resource/absence checks (missing `aws_s3_bucket_public_access_block`,
  versioning) are deferred — they over-flag.

## Deferred (documented, not hand-waved)

- Namespace-level Pod Security Admission labels
  (`pod-security.kubernetes.io/{enforce,audit,warn}`) — a different manifest
  kind (`Namespace`); a follow-up rule.
- Kubernetes RBAC privilege-escalation (`auditing-kubernetes-rbac-privilege-escalation`),
  Dockerfile/container hardening, and the cloud/IaC families remain queued Wave-1
  increments per the triage catalog.

## Proof

`cargo test -p enforcer-lang-security` (unit + `cyberskills_corpus`) and
`cargo test -p enforcer-security --test cyberskills_parity` (the d01
rule-scaffold-parity oracle over the whole cyberskills family, now including
`CYBER-K8S-POD.1`).
