export function formatProofReport(command, result) {
  if (command === "route") return `Proof route: ${result.proofs.length} proof(s), docs=${result.docs.length}`;
  if (command === "run") return `Proof ${result.proofRun.proofId} ${result.proofRun.status}: ${result.proofRun.runId}`;
  if (command === "import-legacy") {
    return `Imported legacy proof ${result.proofRun.proofId} ${result.proofRun.status}: ${result.proofRun.runId}`;
  }
  if (command === "migrate-legacy") {
    return `Generated ${result.generatedProofCount} legacy proof definition(s) for profile ${result.profile}: dryRun=${result.dryRun}`;
  }
  if (command === "parity") return `Proof parity ${result.coverage}: deletionReady=${result.deletionReady}`;
  if (command === "status") return formatStatusRuns(result.runs);
  if (command === "artifact") return result.text ?? result.message ?? "";
  if (command === "diagnostics") return formatDiagnostics(result.diagnostics ?? []);
  return JSON.stringify(result, null, 2);
}

function formatStatusRuns(runs) {
  return runs.map((run) => `${run.runId} ${run.status} ${run.proofId}`).join("\n");
}

function formatDiagnostics(diagnostics) {
  return diagnostics.map((diagnostic) => `${diagnostic.ruleId}: ${diagnostic.message}`).join("\n");
}
