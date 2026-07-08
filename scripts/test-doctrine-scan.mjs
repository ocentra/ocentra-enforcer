#!/usr/bin/env node
/*
 * Standalone entrypoint: scan any target project for what kinds of tests it
 * has and what its nature implies it's missing.
 *
 * Usage:
 *   node scripts/test-doctrine-scan.mjs --root <path> [--json]
 */
import process from "node:process";
import { scanTestDoctrine } from "../src/test-doctrine-scan.mjs";

function parseArgs(argv) {
  const args = { root: process.cwd(), json: false };
  for (let i = 2; i < argv.length; i += 1) {
    if (argv[i] === "--root") args.root = argv[++i];
    else if (argv[i] === "--json") args.json = true;
  }
  return args;
}

function printText(report) {
  console.log(`Test-doctrine scan: ${report.root}`);
  console.log(`Nature: ${JSON.stringify(report.nature.languages)} webApi=${report.nature.isWebApi} frontend=${report.nature.hasFrontendUi} asyncWorkers=${report.nature.hasAsyncWorkers} moneyCritical=${report.nature.hasMoneyCriticalSurface} multiService=${report.nature.hasMultiServiceBoundary}`);
  console.log("");
  console.log(`Present (${report.summary.categoriesPresent}/${report.summary.categoriesRelevant} relevant):`);
  for (const [key, value] of Object.entries(report.detected)) {
    if (value.relevant && value.present) console.log(`  [x] ${value.label} — ${value.evidence[0]}`);
  }
  console.log("");
  console.log(`Missing (${report.missing.length}):`);
  for (const item of report.missing) {
    console.log(`  [ ] (${item.tier}) ${item.label} — ${item.reason}`);
  }
  console.log("");
  console.log(report.caveat);
}

const args = parseArgs(process.argv);
const report = scanTestDoctrine({ root: args.root });
if (args.json) console.log(JSON.stringify(report, null, 2));
else printText(report);
process.exit(report.summary.coreMissing > 0 ? 1 : 0);
