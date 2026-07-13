#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";

function run(command, args, cwd, input) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    input,
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed: ${result.stderr || result.stdout}`);
  }
  return result.stdout;
}

function stagedFiles(root) {
  return run("git", ["diff", "--cached", "--name-only", "--diff-filter=ACMR"], root)
    .split(/\r?\n/u)
    .filter(Boolean);
}

function scan(root, enforcerRoot, files) {
  const result = spawnSync(
    process.execPath,
    [path.join(enforcerRoot, "scripts", "rust-rules.mjs"), "scan", "--root", root, "--json", "--files", ...files],
    { cwd: root, encoding: "utf8" },
  );
  if (!result.stdout) throw new Error(result.stderr || "scanner produced no JSON report");
  return JSON.parse(result.stdout).violations ?? [];
}

function key(finding) {
  // Source text and line numbers legitimately move while an existing debt
  // item is repaired, so the ratchet compares per-rule counts per file.
  return [finding.ruleId, finding.file].join("\u0000");
}

function counts(findings) {
  const result = new Map();
  for (const finding of findings) result.set(key(finding), (result.get(key(finding)) ?? 0) + 1);
  return result;
}

export function increasedFindings(base, candidate) {
  const before = counts(base);
  const after = counts(candidate);
  return [...after].flatMap(([finding, count]) =>
    Array.from({ length: Math.max(0, count - (before.get(finding) ?? 0)) }, () => finding),
  );
}

export function checkStagedRatchet({ root, enforcerRoot = root }) {
  const files = stagedFiles(root);
  if (files.length === 0) return { ok: true, files, increased: [] };
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "ocentra-enforcer-ratchet-"));
  try {
    run("git", ["worktree", "add", "--detach", tempRoot, "HEAD"], root);
    const baseline = scan(tempRoot, enforcerRoot, files);
    for (const file of files) {
      const content = run("git", ["show", `:${file}`], root);
      const target = path.join(tempRoot, file);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, content);
    }
    const candidate = scan(tempRoot, enforcerRoot, files);
    const increased = increasedFindings(baseline, candidate);
    return { ok: increased.length === 0, files, increased };
  } finally {
    spawnSync("git", ["worktree", "remove", "--force", tempRoot], { cwd: root, encoding: "utf8" });
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
}

if (import.meta.url === `file://${process.argv[1].replaceAll("\\", "/")}`) {
  const root = path.resolve(process.argv[2] ?? process.cwd());
  const result = checkStagedRatchet({ root, enforcerRoot: path.resolve(process.argv[3] ?? root) });
  if (!result.ok) {
    process.stderr.write(`ocentra-enforcer pre-commit ratchet rejected ${result.increased.length} new hard finding(s):\n${result.increased.join("\n")}\n`);
    process.exitCode = 1;
  }
}
