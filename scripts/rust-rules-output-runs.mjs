function formatRunMeta(run) {
  const meta = [];
  if (run.crateName) meta.push(`crate=${run.crateName}`);
  if (run.packageName) meta.push(`package=${run.packageName}`);
  if (run.domain) meta.push(`domain=${run.domain}`);
  if (run.tags?.length) meta.push(`tags=${run.tags.join(",")}`);
  return meta.join(" ");
}

function formatRunListEntry(run) {
  const meta = formatRunMeta(run);
  const suffix = meta ? ` ${meta}` : "";
  return `${run.runId} ${run.status} ${run.tool} diagnostics=${run.diagnosticCount}${suffix}`;
}

function printDiagnostics(report) {
  for (const diagnostic of report.diagnostics ?? []) {
    console.log(
      `${diagnostic.file}:${diagnostic.line}: ${diagnostic.ruleId} ${diagnostic.message}`,
    );
  }
}

function printRunList(report) {
  for (const run of report.runs) {
    console.log(formatRunListEntry(run));
  }
}

export function printRunsReport(command, report) {
  if (command === "list") {
    printRunList(report);
    return;
  }
  if (command === "artifact") {
    console.log(report.text ?? report.message ?? "");
    return;
  }
  if (command === "diagnostics") {
    printDiagnostics(report);
    return;
  }
  console.log(JSON.stringify(report, null, 2));
}
