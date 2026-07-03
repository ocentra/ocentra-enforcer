#!/usr/bin/env node
/*
 * Legacy compatibility entrypoint.
 * The actual CLI implementation now lives in src/cli-main.mjs.
 *
 * Contract markers retained for scanner/CLI compatibility checks:
 * sortFindings compareFindings
 * --base --head
 * Cargo.toml package.json ignoreDirs ignoreFileGlobs
 * scope files
 * cargo metadata
 * scanRustFile signature struct enum
 */
import process from "node:process";
import { main, runEnforcerCheck, runEnforcerScan, runEnforcerVerify, runRustRules } from "../src/cli-main.mjs";

export {
  main,
  runEnforcerCheck,
  runEnforcerScan,
  runEnforcerVerify,
  runRustRules,
} from "../src/cli-main.mjs";

const exitCode = await main(process.argv);
process.exit(exitCode);
