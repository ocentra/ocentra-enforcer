import { proofInputSchema } from "./rust-rules-mcp-helpers.mjs";

export const PROOF_TOOLS = [
  {
    name: "ocentra_enforcer_proof_route",
    description:
      "Return compact indexed proof definitions relevant to files, plan, capability, profile, or one proof id.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_run",
    description:
      "Run or record a proof through the Enforcer proof harness, storing local proof artifacts under .enforce/proofs.",
    inputSchema: proofInputSchema({
      proofId: { type: "string" },
      command: { type: "array", items: { type: "string" } },
    }),
  },
  {
    name: "ocentra_enforcer_proof_status",
    description: "Return compact proof run status for the target repository.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_inventory",
    description:
      "Read-only inventory of legacy proof scripts in a target repository, grouped by family/capability.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_import_legacy",
    description:
      "Read legacy proof artifacts and write canonical Enforcer proof runs under .enforce/proofs.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_parity",
    description:
      "Compare legacy proof artifacts with an imported Enforcer proof run and report deletion readiness.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_claim",
    description:
      "Validate that named proof ids support a PR-ready or completion claim without stale/missing artifacts.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_last_failure",
    description:
      "Return the latest failed/manual-required proof with compact diagnostics.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_diagnostics",
    description:
      "Return compact proof diagnostics for the latest or requested proof run.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_artifact",
    description:
      "Return a bounded proof artifact only when compact diagnostics are insufficient.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_reset",
    description:
      "Delete local proof run state under .enforce/proofs for a target repository.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_prune",
    description: "Apply proof retention policy under .enforce/proofs.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_export",
    description:
      "Return a manifest-only proof export suitable for CI artifact upload metadata.",
    inputSchema: proofInputSchema(),
  },
];
