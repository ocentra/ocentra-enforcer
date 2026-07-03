export function printCodexDoctorReport(report) {
  console.log(`Ocentra Enforcer Codex doctor: ${report.ok ? "PASS" : "FAIL"}`);
  console.log(`Pack: ${report.packRoot}`);
  if (report.root) console.log(`Target: ${report.root}`);
  console.log(`Codex config: ${report.codexConfigPath}`);
  for (const check of report.checks) {
    const label = check.ok
      ? "PASS"
      : check.severity === "warning"
        ? "WARN"
        : "FAIL";
    console.log(`${label} ${check.name}: ${check.detail}`);
  }
  console.log("");
  for (const step of report.nextSteps) console.log(`next: ${step}`);
}
