import { spawnSync } from "node:child_process";

export function runTargets(repoRoot, selectedTargets, args) {
  if (args.list || args.noRun) return discoveryResult();
  const failures = [];
  for (const target of selectedTargets) {
    const targetResult = runTarget(repoRoot, target, args.quiet);
    if (targetResult.exitCode !== 0) {
      failures.push(targetResult);
      break;
    }
  }
  return {
    mode: "executed",
    ok: failures.length === 0,
    executedTargets: selectedTargets.length - failures.length,
    failedTargets: failures,
  };
}

function discoveryResult() {
  return {
    mode: "discovery-only",
    ok: true,
    executedTargets: 0,
    failedTargets: [],
  };
}

function runTarget(repoRoot, target, quiet) {
  const cargoArgs = ["test", "-p", "enforcer-memory", "--test", target];
  if (quiet) cargoArgs.push("--quiet");
  cargoArgs.push("-j", "1");
  const startedAt = Date.now();
  const result = spawnSync("cargo", cargoArgs, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: "inherit",
  });
  return {
    target,
    exitCode: result.status ?? 1,
    durationMs: Date.now() - startedAt,
  };
}
