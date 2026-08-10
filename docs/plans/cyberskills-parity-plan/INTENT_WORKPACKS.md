# Intent-Driven CyberSkills Workpacks

This is the execution layer between the 817-row source ledger and CP00–CP13. It uses canonical intent families and deterministic bounded action packets instead of duplicating 816 prose files.

## Mechanical contract

- `CYBERSKILLS_INTENT_MATRIX.json` is the machine-readable classification and packet source.
- It covers 816 available catalog IDs exactly once at family level; the protected fileless-malware identity is excluded and never read.
- Native implementation packets are derived to CP09 or CP12 and capped at five or one skill respectively.
- Advisory/manual retention packets are derived to CP11 in batches of ten.
- The external-engine component remains explicitly blocked/reference-only under the native-product decision.
- Every packet requires source hash/anchor verification, positive/negative/malformed/boundary evidence as applicable, Enforcer dogfood, and explicit `notProved`.
- A CP08 decomposition row is evidence for identity/component accounting only; it cannot make a packet implemented or proved.

## Family inventory

| Family | Skills | Native route | Native batch | Retention route | Capability tracks |
|---|---:|---|---:|---|---|
| `IF/ai-security` | 14 | CP09 | 5 | CP11/10 | native-static, native-safe-simulation, advisory-manual |
| `IF/api-security` | 28 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, advisory-manual |
| `IF/blockchain-security` | 2 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, advisory-manual |
| `IF/cloud-security` | 66 | CP12 | 1 | CP11/10 | native-static, native-offline-artifact, repository-graph, advisory-manual |
| `IF/compliance-governance` | 10 | CP09 | 5 | CP11/10 | native-static, advisory-manual |
| `IF/container-security` | 33 | CP12 | 1 | CP11/10 | native-static, native-offline-artifact, repository-graph, advisory-manual |
| `IF/cryptography` | 16 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, advisory-manual |
| `IF/data-protection` | 1 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, advisory-manual |
| `IF/deception-technology` | 6 | CP09 | 5 | CP11/10 | native-static, native-safe-simulation, advisory-manual |
| `IF/devsecops` | 18 | CP12 | 1 | CP11/10 | native-static, repository-graph, advisory-manual |
| `IF/digital-forensics` | 41 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, advisory-manual |
| `IF/endpoint-security` | 17 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, advisory-manual |
| `IF/hardware-firmware-security` | 6 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, advisory-manual |
| `IF/identity-access-management` | 40 | CP12 | 1 | CP11/10 | native-static, repository-graph, advisory-manual |
| `IF/incident-response` | 26 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, advisory-manual |
| `IF/malware-analysis` | 38 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, native-safe-simulation, advisory-manual |
| `IF/mobile-security` | 13 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, native-safe-simulation, advisory-manual |
| `IF/network-security` | 43 | CP12 | 1 | CP11/10 | native-static, native-offline-artifact, repository-graph, advisory-manual |
| `IF/ot-ics-security` | 29 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, native-safe-simulation, advisory-manual |
| `IF/penetration-testing` | 23 | CP09 | 5 | CP11/10 | native-static, native-safe-simulation, advisory-manual |
| `IF/phishing-defense` | 16 | CP09 | 5 | CP11/10 | native-static, native-safe-simulation, advisory-manual |
| `IF/privacy-compliance` | 2 | CP09 | 5 | CP11/10 | native-static, advisory-manual |
| `IF/purple-team` | 1 | CP09 | 5 | CP11/10 | native-static, native-safe-simulation, advisory-manual |
| `IF/ransomware-defense` | 13 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, advisory-manual |
| `IF/red-teaming` | 35 | CP09 | 5 | CP11/10 | native-static, native-safe-simulation, advisory-manual |
| `IF/soc-operations` | 63 | CP12 | 1 | CP11/10 | native-static, repository-graph, advisory-manual |
| `IF/supply-chain-security` | 8 | CP12 | 1 | CP11/10 | native-static, native-offline-artifact, repository-graph, advisory-manual |
| `IF/threat-detection` | 7 | CP12 | 1 | CP11/10 | native-static, native-offline-artifact, repository-graph, advisory-manual |
| `IF/threat-hunting` | 58 | CP12 | 1 | CP11/10 | native-static, native-offline-artifact, repository-graph, advisory-manual |
| `IF/threat-intelligence` | 52 | CP12 | 1 | CP11/10 | native-static, native-offline-artifact, repository-graph, advisory-manual |
| `IF/vulnerability-management` | 25 | CP12 | 1 | CP11/10 | native-static, native-offline-artifact, repository-graph, advisory-manual |
| `IF/web-application-security` | 46 | CP12 | 1 | CP11/10 | native-static, native-offline-artifact, repository-graph, advisory-manual |
| `IF/wireless-security` | 2 | CP09 | 5 | CP11/10 | native-static, native-offline-artifact, native-safe-simulation, advisory-manual |
| `IF/zero-trust-architecture` | 18 | CP12 | 1 | CP11/10 | native-static, native-offline-artifact, repository-graph, advisory-manual |

Total: 816 available skills, 34 canonical intent families (from 46 raw subdomain values).

## Execution order

1. CP00 establishes typed source and coverage truth.
2. CP08 supplies immutable component decomposition.
3. CP05/CP03 establish the native predicate and repository-fact contracts.
4. `graph next` selects only a packet whose hard dependencies are derived `done`.
5. Each packet is claimed exactly, implemented in Rust or explicitly retained, dogfooded through Enforcer, and evidenced before the next selection.
6. CP13 closes only when all packet/component contracts are independently green; no blended percentage or decomposition-only completion is accepted.

The complete skill-ID assignments, static Python audit summary, reuse catalog, routes, dependencies, and non-proofs are in the JSON matrix.
