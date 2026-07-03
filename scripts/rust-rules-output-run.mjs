export function printRunReport(report) {
  console.log(
    `Ocentra Enforcer run ${report.summary.status}: ${report.summary.runId}`,
  );
  console.log(`Tool: ${report.summary.tool}`);
  if (report.summary.crateName)
    console.log(`Crate: ${report.summary.crateName}`);
  if (report.summary.packageName)
    console.log(`Package: ${report.summary.packageName}`);
  if (report.summary.domain) console.log(`Domain: ${report.summary.domain}`);
  console.log(`Exit: ${report.summary.exitCode}`);
  console.log(`Diagnostics: ${report.summary.diagnosticCount}`);
  console.log(
    `Artifacts: ${Object.values(report.summary.artifacts).join(", ")}`,
  );
  for (const diagnostic of report.diagnostics.slice(0, 10)) {
    console.log(
      `${diagnostic.file}:${diagnostic.line}: ${diagnostic.ruleId} ${diagnostic.message}`,
    );
  }
}
