#!/usr/bin/env node
/*
 * Desktop bridge for analyses that still live in the legacy Node pack. This
 * emits a stable, discriminated JSON envelope and deliberately exits zero for
 * report findings: findings are the report, not an execution failure.
 *
 * Usage:
 *   node scripts/desktop-analysis.mjs --root <path> --kind <test-doctrine|ui-logic-coupling>
 */
import process from "node:process";
import { scanTestDoctrine } from "../src/test-doctrine-scan.mjs";
import { scanUiLogicCoupling } from "../src/ui-logic-coupling-scan.mjs";

function parseArgs(argv) {
  const args = { root: "", kind: "" };
  for (let index = 2; index < argv.length; index += 1) {
    if (argv[index] === "--root") args.root = argv[++index] ?? "";
    else if (argv[index] === "--kind") args.kind = argv[++index] ?? "";
  }
  return args;
}

function fail(message) {
  process.stderr.write(`desktop analysis: ${message}\n`);
  process.exitCode = 2;
}

const args = parseArgs(process.argv);
if (!args.root || !args.kind) {
  fail("--root and --kind are required");
} else {
  try {
    const report = args.kind === "test-doctrine"
      ? scanTestDoctrine({ root: args.root })
      : args.kind === "ui-logic-coupling"
        ? scanUiLogicCoupling({ root: args.root })
        : null;
    if (!report) {
      fail(`unsupported analysis kind: ${args.kind}`);
    } else {
      process.stdout.write(`${JSON.stringify({ schemaVersion: 1, analysisKind: args.kind, report })}\n`);
    }
  } catch (error) {
    fail(error instanceof Error ? error.stack ?? error.message : String(error));
  }
}
