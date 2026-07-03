import {
  printCheckReport as printCheckReportImpl,
  printScanReport as printScanReportImpl,
} from "./rust-rules-output-check.mjs";
import {
  printRunReport as printRunReportImpl,
} from "./rust-rules-output-run.mjs";
import {
  printRunsReport as printRunsReportImpl,
} from "./rust-rules-output-runs.mjs";
import {
  printCodexDoctorReport as printCodexDoctorReportImpl,
} from "./rust-rules-output-codex-doctor.mjs";
import {
  printCodexInstallReport as printCodexInstallReportImpl,
} from "./rust-rules-output-codex-install.mjs";
import {
  printCodexUninstallReport as printCodexUninstallReportImpl,
} from "./rust-rules-output-codex-uninstall.mjs";
import { printInitReport as printInitReportImpl } from "./rust-rules-output-init.mjs";

export function printCheckReport(report) {
  return printCheckReportImpl(report);
}

export function printScanReport(report) {
  return printScanReportImpl(report);
}

export function printRunReport(report) {
  return printRunReportImpl(report);
}

export function printRunsReport(command, report) {
  return printRunsReportImpl(command, report);
}

export function printCodexDoctorReport(report) {
  return printCodexDoctorReportImpl(report);
}

export function printCodexInstallReport(report) {
  return printCodexInstallReportImpl(report);
}

export function printCodexUninstallReport(report) {
  return printCodexUninstallReportImpl(report);
}

export function printInitReport(report) {
  return printInitReportImpl(report);
}
