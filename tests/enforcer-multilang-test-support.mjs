import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnCli } from "./cli-spawn.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SCRIPT = path.join(ROOT, "scripts", "rust-rules.mjs");
const TEST_CLI_MAX_BUFFER = 32 * 1024 * 1024;

export function makeProject(files) {
  const dir = fs.mkdtempSync(
    path.join(os.tmpdir(), "ocentra-enforcer-multilang-"),
  );
  for (const [rel, content] of Object.entries(files)) {
    const full = path.join(dir, rel);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    fs.writeFileSync(full, content.trimStart(), "utf8");
  }
  return dir;
}

export function run(project, args) {
  return spawnCli(process.execPath, [SCRIPT, ...args, "--root", project], {
    encoding: "utf8",
    maxBuffer: TEST_CLI_MAX_BUFFER,
  });
}

export function parseReport(result) {
  return JSON.parse(result.stdout);
}

export const pythonDoubleImport = [
  "from unittest.",
  "m",
  "ock import M",
  "ock",
].join("");
export const pythonDoubleCall = ["M", "ock()"].join("");
