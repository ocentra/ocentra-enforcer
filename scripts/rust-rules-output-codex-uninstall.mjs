export function printCodexUninstallReport(report) {
  console.log(`Ocentra Enforcer Codex uninstall`);
  console.log(`Dry run: ${report.dryRun ? "yes" : "no"}`);
  console.log("");
  console.log(
    `${report.changed ? (report.dryRun ? "would-write" : "write") : "skip-missing"} ${report.codexConfigPath}`,
  );
  console.log(
    `${report.skillChanged ? (report.dryRun ? "would-remove" : "remove") : "skip-missing"} ${report.skillTarget}`,
  );
  console.log(
    `${report.globalAgentsChanged ? (report.dryRun ? "would-write" : "write") : "skip-missing"} ${report.globalAgentsPath}`,
  );
  if (report.backupPath) console.log(`backup ${report.backupPath}`);
  if (report.globalAgentsBackupPath) console.log(`backup ${report.globalAgentsBackupPath}`);
}
