#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const packRoot = path.resolve(path.join(path.dirname(fileURLToPath(import.meta.url)), ".."));

function parseArgs(argv) {
  const parsed = {
    root: process.cwd(),
    profile: "strict",
    script: null,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--root") parsed.root = argv[++index] ?? parsed.root;
    else if (token === "--profile") parsed.profile = argv[++index] ?? parsed.profile;
    else if (token === "--script") parsed.script = argv[++index] ?? null;
    else throw new Error(`Unknown profile proof runner argument: ${token}`);
  }
  if (!parsed.script) throw new Error("--script is required");
  return parsed;
}

function copyMissingProfileScripts({ root, profile }) {
  const sourceRoot = path.join(packRoot, "profiles", profile, "legacy-scripts", "scripts", "test");
  const targetRoot = path.join(root, "scripts", "test");
  if (!fs.existsSync(sourceRoot)) {
    throw new Error(`Profile legacy script root is missing: ${sourceRoot}`);
  }
  const created = [];
  fs.mkdirSync(targetRoot, { recursive: true });
  for (const source of collectFiles(sourceRoot)) {
    const relative = path.relative(sourceRoot, source);
    const target = path.join(targetRoot, relative);
    if (fs.existsSync(target)) continue;
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.copyFileSync(source, target);
    created.push(target);
  }
  return created;
}

function collectFiles(root) {
  const files = [];
  const stack = [root];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const next = path.join(current, entry.name);
      if (entry.isDirectory()) stack.push(next);
      else if (entry.isFile()) files.push(next);
    }
  }
  return files.sort((left, right) => left.localeCompare(right));
}

function cleanupCreatedFiles(files) {
  for (const file of files.sort((left, right) => right.length - left.length)) {
    try {
      fs.rmSync(file, { force: true });
    } catch {
      // Best effort cleanup. The proof run output will still show the command result.
    }
  }
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const root = path.resolve(args.root);
  const created = copyMissingProfileScripts({ root, profile: args.profile });
  const scriptPath = path.join(root, args.script);
  try {
    const result = spawnSync(process.execPath, [scriptPath], {
      cwd: root,
      env: {
        ...process.env,
        OCENTRA_ENFORCER_PROFILE: args.profile,
        OCENTRA_ENFORCER_TARGET_ROOT: root,
      },
      stdio: "inherit",
      windowsHide: true,
    });
    if (result.error) throw result.error;
    process.exitCode = result.status ?? 1;
  } finally {
    cleanupCreatedFiles(created);
  }
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
