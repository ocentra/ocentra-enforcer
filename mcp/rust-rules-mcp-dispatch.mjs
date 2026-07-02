import process from "node:process";
import {
  decodeCheckToolArguments,
  decodeCoordinationToolArguments,
  decodeDoctorToolArguments,
  decodeExplainToolArguments,
  decodeRouteRequest,
  decodeRunQueryArguments,
  decodeRunToolArguments,
  decodeScanToolArguments,
} from "../schemas/effect/enforcer-schemas.mjs";
import {
  MCP_FINGERPRINT_FILES,
  PACKAGE_JSON,
  SERVER_ROOT,
  SERVER_STARTED_AT,
  buildMcpFingerprint,
  changedFingerprintFiles,
  mcpStaleError,
  normalizeToolName,
  shouldBlockStaleMcpTool,
} from "./rust-rules-mcp-helpers.mjs";
import { TOOL_SCHEMAS } from "./rust-rules-mcp-tool-registry.mjs";
import { coordinationHashCompatibility } from "../src/coordination/vendor/events.js";
import {
  coordinationClaim,
  coordinationCloseout,
  coordinationCompact,
  coordinationEnsure,
  coordinationGuard,
  coordinationHealth,
  coordinationIndex,
  coordinationInbox,
  coordinationInit,
  coordinationMail,
  coordinationMessage,
  coordinationNotify,
  coordinationPeer,
  coordinationPresence,
  coordinationRelease,
  coordinationRepair,
  coordinationReport,
  coordinationStatus,
  coordinationStreams,
  coordinationSync,
  coordinationTasks,
  coordinationWorkers,
} from "../src/coordination/api.mjs";
import {
  lastFailure,
  pruneRuns,
  readArtifact,
  resetRuns,
  runDiagnostics,
  runHarness,
  runSummary,
} from "../src/harness.mjs";
import {
  claimProof,
  importLegacyProof,
  inventoryProofs,
  proofArtifact,
  proofDiagnostics,
  proofExport,
  proofLastFailure,
  proofParity,
  proofPrune,
  proofReset,
  proofStatus,
  routeProofs,
  runProof,
} from "../src/proof.mjs";
import { routeRules } from "./rust-rules-mcp-route.mjs";
import {
  latestValidationSummary,
  runCli,
} from "./rust-rules-mcp-runner.mjs";

const STARTUP_FINGERPRINT = buildMcpFingerprint(MCP_FINGERPRINT_FILES);
const TOOL_HANDLERS = new Map([
  ["ocentra_enforcer_route", routeTool],
  ["ocentra_enforcer_scan", scanTool],
  ["ocentra_enforcer_check", checkTool],
  ["ocentra_enforcer_doctor", doctorTool],
  ["ocentra_enforcer_explain", explainTool],
  ["ocentra_enforcer_run", runTool],
  ["ocentra_enforcer_run_status", runStatusTool],
  ["ocentra_enforcer_diagnostics", diagnosticsTool],
  ["ocentra_enforcer_last_failure", lastFailureTool],
  ["ocentra_enforcer_artifact", artifactTool],
  ["ocentra_enforcer_prune_runs", pruneRunsTool],
  ["ocentra_enforcer_reset_runs", resetRunsTool],
  ["ocentra_enforcer_proof_route", proofRouteTool],
  ["ocentra_enforcer_proof_run", proofRunTool],
  ["ocentra_enforcer_proof_status", proofStatusTool],
  ["ocentra_enforcer_proof_inventory", proofInventoryTool],
  ["ocentra_enforcer_proof_import_legacy", proofImportLegacyTool],
  ["ocentra_enforcer_proof_parity", proofParityTool],
  ["ocentra_enforcer_proof_claim", proofClaimTool],
  ["ocentra_enforcer_proof_last_failure", proofLastFailureTool],
  ["ocentra_enforcer_proof_diagnostics", proofDiagnosticsTool],
  ["ocentra_enforcer_proof_artifact", proofArtifactTool],
  ["ocentra_enforcer_proof_reset", proofResetTool],
  ["ocentra_enforcer_proof_prune", proofPruneTool],
  ["ocentra_enforcer_proof_export", proofExportTool],
  ["ocentra_enforcer_coordination_health", coordinationHealthTool],
  ["ocentra_enforcer_coordination_init", coordinationInitTool],
  ["ocentra_enforcer_coordination_presence", coordinationPresenceTool],
  ["ocentra_enforcer_coordination_index", coordinationIndexTool],
  ["ocentra_enforcer_coordination_streams", coordinationStreamsTool],
  ["ocentra_enforcer_coordination_sync", coordinationSyncTool],
  ["ocentra_enforcer_coordination_peer", coordinationPeerTool],
  ["ocentra_enforcer_coordination_ensure", coordinationEnsureTool],
  ["ocentra_enforcer_coordination_compact", coordinationCompactTool],
  ["ocentra_enforcer_coordination_notify", coordinationNotifyTool],
  ["ocentra_enforcer_coordination_mail", coordinationMailTool],
  ["ocentra_enforcer_coordination_status", coordinationStatusTool],
  ["ocentra_enforcer_coordination_inbox", coordinationInboxTool],
  ["ocentra_enforcer_coordination_claim", coordinationClaimTool],
  ["ocentra_enforcer_coordination_release", coordinationReleaseTool],
  ["ocentra_enforcer_coordination_closeout", coordinationCloseoutTool],
  ["ocentra_enforcer_coordination_repair", coordinationRepairTool],
  ["ocentra_enforcer_coordination_guard", coordinationGuardTool],
  ["ocentra_enforcer_coordination_report", coordinationReportTool],
  ["ocentra_enforcer_coordination_message", coordinationMessageTool],
  ["ocentra_enforcer_coordination_workers", coordinationWorkersTool],
  ["ocentra_enforcer_coordination_tasks", coordinationTasksTool],
]);

export async function callTool(params) {
  const name = normalizeToolName(params.name);
  const args = params.arguments ?? {};
  try {
    rejectUnexpectedArguments(name, args);
    if (name === "ocentra_enforcer_mcp_status") {
      return toolJson(mcpStatus());
    }
    const freshness = mcpStatus();
    if (shouldBlockStaleMcpTool(name, args, freshness)) {
      return toolJson(mcpStaleError(name, freshness, args));
    }
    const handler = TOOL_HANDLERS.get(name);
    if (!handler) {
      return toolError(`Unknown tool: ${params.name}`);
    }
    return await handler(args);
  } catch (error) {
    return toolError(error instanceof Error ? error.message : String(error));
  }
}

function routeTool(args) {
  return toolJson(routeRules(decodeRouteRequest(args)));
}

function scanTool(args) {
  const decoded = decodeScanToolArguments(args);
  return runCli(decoded.cargo ? "cargo" : "scan", decoded);
}

function checkTool(args) {
  return runCli("check", decodeCheckToolArguments(args));
}

function doctorTool(args) {
  return runCli("doctor", decodeDoctorToolArguments(args));
}

function explainTool(args) {
  return runCli("explain", decodeExplainToolArguments(args));
}

function runTool(args) {
  return toolJson(runHarness(decodeRunToolArguments(args)));
}

function runStatusTool(args) {
  const decoded = decodeRunQueryArguments(args);
  const summary = runSummary(decoded);
  const validationSummary = latestValidationSummary(decoded);
  const artifact = summary && decoded.artifact ? readArtifact(decoded) : undefined;
  return toolJson({
    ok: true,
    summary: summary ?? validationSummary,
    summaryType: summary
      ? "harness"
      : validationSummary
        ? "validation"
        : "none",
    validationSummary,
    ...(artifact === undefined ? {} : { artifact }),
  });
}

function diagnosticsTool(args) {
  return toolJson(runDiagnostics(decodeRunQueryArguments(args)));
}

function lastFailureTool(args) {
  return toolJson(lastFailure(decodeRunQueryArguments(args)));
}

function artifactTool(args) {
  return toolJson(readArtifact(decodeRunQueryArguments(args)));
}

function pruneRunsTool(args) {
  return toolJson(pruneRuns(decodeRunQueryArguments(args)));
}

function resetRunsTool(args) {
  return toolJson(resetRuns(decodeRunQueryArguments(args)));
}

function proofRouteTool(args) {
  return toolJson(routeProofs(args, SERVER_ROOT));
}

function proofRunTool(args) {
  return toolJson(runProof(args, SERVER_ROOT));
}

function proofStatusTool(args) {
  return toolJson(proofStatus(args));
}

function proofInventoryTool(args) {
  return toolJson(inventoryProofs(args));
}

function proofImportLegacyTool(args) {
  return toolJson(importLegacyProof(args, SERVER_ROOT));
}

function proofParityTool(args) {
  return toolJson(proofParity(args));
}

function proofClaimTool(args) {
  return toolJson(claimProof(args, SERVER_ROOT));
}

function proofLastFailureTool(args) {
  return toolJson(proofLastFailure(args));
}

function proofDiagnosticsTool(args) {
  return toolJson(proofDiagnostics(args));
}

function proofArtifactTool(args) {
  return toolJson(proofArtifact(args));
}

function proofResetTool(args) {
  return toolJson(proofReset(args));
}

function proofPruneTool(args) {
  return toolJson(proofPrune(args));
}

function proofExportTool(args) {
  return toolJson(proofExport(args));
}

function coordinationHealthTool(args) {
  return toolJsonAsync(coordinationHealth(decodeCoordinationToolArguments(args)));
}

function coordinationInitTool(args) {
  return toolJsonAsync(coordinationInit(decodeCoordinationToolArguments(args)));
}

function coordinationPresenceTool(args) {
  return toolJsonAsync(coordinationPresence(decodeCoordinationToolArguments(args)));
}

function coordinationIndexTool(args) {
  return toolJsonAsync(coordinationIndex(decodeCoordinationToolArguments(args)));
}

function coordinationStreamsTool(args) {
  return toolJsonAsync(coordinationStreams(decodeCoordinationToolArguments(args)));
}

function coordinationSyncTool(args) {
  return toolJsonAsync(coordinationSync(decodeCoordinationToolArguments(args)));
}

function coordinationPeerTool(args) {
  return toolJsonAsync(coordinationPeer(decodeCoordinationToolArguments(args)));
}

function coordinationEnsureTool(args) {
  return toolJsonAsync(coordinationEnsure(decodeCoordinationToolArguments(args)));
}

function coordinationCompactTool(args) {
  return toolJsonAsync(coordinationCompact(decodeCoordinationToolArguments(args)));
}

function coordinationNotifyTool(args) {
  return toolJsonAsync(coordinationNotify(decodeCoordinationToolArguments(args)));
}

function coordinationMailTool(args) {
  return toolJsonAsync(coordinationMail(decodeCoordinationToolArguments(args)));
}

function coordinationStatusTool(args) {
  return toolJsonAsync(coordinationStatus(decodeCoordinationToolArguments(args)));
}

function coordinationInboxTool(args) {
  return toolJsonAsync(coordinationInbox(decodeCoordinationToolArguments(args)));
}

function coordinationClaimTool(args) {
  return toolJsonAsync(coordinationClaim(decodeCoordinationToolArguments(args)));
}

function coordinationReleaseTool(args) {
  return toolJsonAsync(coordinationRelease(decodeCoordinationToolArguments(args)));
}

function coordinationCloseoutTool(args) {
  return toolJsonAsync(coordinationCloseout(decodeCoordinationToolArguments(args)));
}

function coordinationRepairTool(args) {
  return toolJsonAsync(coordinationRepair(decodeCoordinationToolArguments(args)));
}

function coordinationGuardTool(args) {
  return toolJsonAsync(coordinationGuard(decodeCoordinationToolArguments(args)));
}

function coordinationReportTool(args) {
  return toolJsonAsync(coordinationReport(decodeCoordinationToolArguments(args)));
}

function coordinationMessageTool(args) {
  return toolJsonAsync(coordinationMessage(decodeCoordinationToolArguments(args)));
}

function coordinationWorkersTool(args) {
  return toolJsonAsync(coordinationWorkers(decodeCoordinationToolArguments(args)));
}

function coordinationTasksTool(args) {
  return toolJsonAsync(coordinationTasks(decodeCoordinationToolArguments(args)));
}

function toolJsonAsync(promise) {
  return promise.then(toolJson, (error) =>
    toolError(error instanceof Error ? error.message : String(error)),
  );
}

function toolJson(value) {
  return {
    isError: value?.ok === false,
    content: [{ type: "text", text: JSON.stringify(value, null, 2) }],
  };
}

function mcpStatus() {
  const current = buildMcpFingerprint(MCP_FINGERPRINT_FILES);
  const stale = STARTUP_FINGERPRINT.digest !== current.digest;
  const hashCompatibility = coordinationHashCompatibility();
  const hashCompatible = hashCompatibility.ok;
  const directWritesAllowed = !stale && hashCompatible;
  return {
    ok: directWritesAllowed,
    stale,
    reloadRequired: !directWritesAllowed,
    writeCompatible: directWritesAllowed,
    directWritesAllowed,
    hashCompatible,
    hashCompatibility,
    packRoot: SERVER_ROOT,
    processId: process.pid,
    startedAt: SERVER_STARTED_AT,
    nodeVersion: process.version,
    runningVersion: PACKAGE_JSON.version,
    currentVersion: current.packageVersion,
    startup: STARTUP_FINGERPRINT,
    current,
    changedFiles: changedFingerprintFiles(
      STARTUP_FINGERPRINT.files,
      current.files,
    ),
    nextStep: stale
      ? "Restart Codex Desktop/MCP or use ocentra_enforcer_run to invoke the updated CLI from the pack root."
      : hashCompatible
        ? "MCP server fingerprint matches the current Enforcer files and coordination hash compatibility is valid."
        : "Restart Codex Desktop/MCP or use ocentra_enforcer_run; coordination hash compatibility failed.",
  };
}

function rejectUnexpectedArguments(name, args) {
  if (!args || typeof args !== "object" || Array.isArray(args)) {
    return;
  }
  const schema = TOOL_SCHEMAS.get(name);
  if (!schema || schema.additionalProperties !== false) {
    return;
  }
  const allowed = new Set(Object.keys(schema.properties ?? {}));
  const unexpected = Object.keys(args).filter((key) => !allowed.has(key));
  if (unexpected.length === 0) {
    return;
  }
  throw new Error(
    `${name} unexpected argument(s): ${unexpected.sort((left, right) => left.localeCompare(right)).join(", ")}`,
  );
}

function toolError(message) {
  const body = {
    ok: false,
    error: String(message),
    message: String(message),
  };
  return {
    isError: true,
    content: [{ type: "text", text: JSON.stringify(body, null, 2) }],
  };
}
