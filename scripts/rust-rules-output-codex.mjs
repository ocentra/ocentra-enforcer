export function printInitReport(report) {
  const mode = report.dryRun ? "dry-run" : "write";
  console.log(`Ocentra Enforcer init ${mode} for ${report.root}`);
  console.log(`Profile: ${report.profile}`);
  console.log(`Adapters: ${report.adapters.join(", ")}`);
  for (const file of report.files) {
    console.log(`${file.action} ${file.path} (${file.adapter})`);
  }
}

import { printCodexDoctorReport as printCodexDoctorReportImpl } from "./rust-rules-output-codex-doctor.mjs";
import { printCodexInstallReport as printCodexInstallReportImpl } from "./rust-rules-output-codex-install.mjs";
import { printCodexUninstallReport as printCodexUninstallReportImpl } from "./rust-rules-output-codex-uninstall.mjs";

export function printCodexInstallReport(report) {
  return printCodexInstallReportImpl(report);
}

export function printCodexUninstallReport(report) {
  return printCodexUninstallReportImpl(report);
}

export function printCodexDoctorReport(report) {
  return printCodexDoctorReportImpl(report);
}
