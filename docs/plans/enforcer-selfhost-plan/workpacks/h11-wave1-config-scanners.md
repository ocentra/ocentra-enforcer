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
