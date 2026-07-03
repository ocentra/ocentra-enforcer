export function printCodexInstallReport(report) {
  console.log(
    report.root
      ? `Ocentra Enforcer Codex install for ${report.root}`
      : "Ocentra Enforcer Codex global install",
  );
  console.log(`Profile: ${report.profile}`);
  console.log(`Dry run: ${report.dryRun ? "yes" : "no"}`);
  console.log("");
  if (report.target) {
    console.log("Target repo wiring:");
    for (const file of report.target.files) {
      console.log(`${file.action} ${file.path} (${file.adapter})`);
    }
  } else {
    console.log("Target repo wiring: skipped (no --root passed)");
  }
  console.log("");
  console.log("Codex global MCP:");
  const action = report.codexMcp.changed
    ? report.dryRun
      ? "would-write"
      : "write"
    : "skip-existing";
  console.log(`${action} ${report.codexMcp.codexConfigPath}`);
  console.log(
    `server ${report.codexMcp.serverName}: node ${report.codexMcp.serverPath}`,
  );
  console.log(`ledger root: ${report.codexMcp.ledgerRoot}`);
  if (report.codexMcp.backupPath)
    console.log(`backup ${report.codexMcp.backupPath}`);
  for (const check of report.codexMcp.checks) {
    console.log(`${check.ok ? "PASS" : "FAIL"} ${check.name}: ${check.detail}`);
  }
  console.log("");
  console.log("Codex user skill:");
  console.log(
    `${report.codexMcp.skillChanged ? (report.dryRun ? "would-write" : "write") : "skip-existing"} ${report.codexMcp.skillTarget}`,
  );
  console.log("");
  console.log("Codex global AGENTS.md:");
  console.log(
    `${report.codexMcp.globalAgentsChanged ? (report.dryRun ? "would-write" : "write") : "skip-existing"} ${report.codexMcp.globalAgentsPath}`,
  );
  console.log("");
  console.log(
    "Restart Codex Desktop or start a new Codex thread after install so the MCP server list refreshes.",
  );
}
