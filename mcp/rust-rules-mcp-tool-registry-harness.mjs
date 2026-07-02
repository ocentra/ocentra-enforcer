import { runQueryInputSchema } from "./rust-rules-mcp-helpers.mjs";
import { harnessRunInputSchema } from "./rust-rules-mcp-tool-registry-shared.mjs";

export const HARNESS_TOOLS = [
  {
    name: "ocentra_enforcer_run",
    description:
      "Run a command through the Enforcer harness, persist raw logs, emit NDJSON diagnostics, and return a compact summary.",
    inputSchema: harnessRunInputSchema(),
  },
  {
    name: "ocentra_enforcer_run_status",
    description: "Return the latest or requested Enforcer harness run summary.",
    inputSchema: runQueryInputSchema(),
  },
  {
    name: "ocentra_enforcer_diagnostics",
    description:
      "Return compact diagnostics for the latest or requested harness run.",
    inputSchema: runQueryInputSchema(),
  },
  {
    name: "ocentra_enforcer_last_failure",
    description:
      "Return the latest failed harness run with compact diagnostics.",
    inputSchema: runQueryInputSchema(),
  },
  {
    name: "ocentra_enforcer_artifact",
    description:
      "Return a bounded raw harness artifact only when compact diagnostics are insufficient.",
    inputSchema: runQueryInputSchema(),
  },
  {
    name: "ocentra_enforcer_prune_runs",
    description:
      "Apply target repo harness retention policy without deleting the whole store.",
    inputSchema: runQueryInputSchema(),
  },
  {
    name: "ocentra_enforcer_reset_runs",
    description: "Delete harness run artifacts for a target root.",
    inputSchema: runQueryInputSchema(),
  },
];
