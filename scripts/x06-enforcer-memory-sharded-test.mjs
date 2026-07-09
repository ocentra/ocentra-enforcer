#!/usr/bin/env node
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { parseArgs } from "./x06-shards/args.mjs";
import { discoverTargets, selectTargets } from "./x06-shards/discovery.mjs";
import { buildProof } from "./x06-shards/proof.mjs";
import { runTargets } from "./x06-shards/runner.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(scriptPath), "..");

function main() {
  const args = parseArgs(process.argv.slice(2));
  const allTargets = discoverTargets(repoRoot);
  const selectedTargets = selectTargets(allTargets, args);
  const result = runTargets(repoRoot, selectedTargets, args);
  const proof = buildProof(allTargets, selectedTargets, args, result);
  writeProof(repoRoot, args.writeProof, proof);
  process.stdout.write(`${JSON.stringify(proof, null, 2)}\n`);
  if (!result.ok) process.exitCode = 1;
}

function writeProof(root, proofPath, proof) {
  if (!proofPath) return;
  const resolved = resolve(root, proofPath);
  mkdirSync(dirname(resolved), { recursive: true });
  writeFileSync(resolved, `${JSON.stringify(proof, null, 2)}\n`);
}

try {
  main();
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}
