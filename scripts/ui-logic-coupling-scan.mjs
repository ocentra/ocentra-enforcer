#!/usr/bin/env node
/*
 * Standalone entrypoint: scan a target project for presentation-layer files
 * (pages/components/views) that call business-logic/API modules directly
 * instead of going through a hook/composable layer.
 *
 * Usage:
 *   node scripts/ui-logic-coupling-scan.mjs --root <path> [--json]
 */
import process from "node:process";
import { scanUiLogicCoupling } from "../src/ui-logic-coupling-scan.mjs";

function parseArgs(argv) {
  const args = { root: process.cwd(), json: false };
  for (let i = 2; i < argv.length; i += 1) {
    if (argv[i] === "--root") args.root = argv[++i];
    else if (argv[i] === "--json") args.json = true;
  }
  return args;
}

function printText(report) {
  console.log(`UI/logic coupling scan: ${report.root}`);
  console.log(`Total findings: ${report.summary.totalFindings} (${report.summary.hardFindings} hard, ${report.summary.infoFindings} info) across ${report.summary.filesWithHardFindings} files`);
  console.log("");
  console.log("Hard findings (business logic called directly from presentation):");
  for (const f of report.hard) {
    console.log(`  [${f.kind}] ${f.file} — imports "${f.binding}" from "${f.source}"${f.hasDataFetchPrimitive ? " (inline useQuery/useMutation)" : ""}`);
  }
  console.log("");
  console.log("Info findings (likely benign — e.g. error-type narrowing):");
  for (const f of report.info) {
    console.log(`  [${f.kind}] ${f.file} — imports "${f.binding}" from "${f.source}"`);
  }
  console.log("");
  console.log(report.caveat);
}

const args = parseArgs(process.argv);
const report = scanUiLogicCoupling({ root: args.root });
if (args.json) console.log(JSON.stringify(report, null, 2));
else printText(report);
process.exit(report.summary.hardFindings > 0 ? 1 : 0);
