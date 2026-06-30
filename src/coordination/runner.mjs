import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const VENDOR_CLI = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "vendor",
  "cli.js",
);

export async function runCoordinationCli(rawArgs = process.argv.slice(2)) {
  const { args, hub, stateRoot } = parseGlobalCoordinationArgs(rawArgs);
  if (args[0] === "health") {
    const { coordinationHealth } = await import("./api.mjs");
    const result = await coordinationHealth({
      ...parseApiArgs(args.slice(1)),
      ...(hub ? { hub } : {}),
      ...(stateRoot ? { stateRoot } : {}),
    });
    console.log(JSON.stringify(result, null, 2));
    return;
  }
  if (args[0] === "guard") {
    const { coordinationGuard } = await import("./api.mjs");
    const result = await coordinationGuard({
      ...parseApiArgs(args.slice(1)),
      ...(hub ? { hub } : {}),
      ...(stateRoot ? { stateRoot } : {}),
    });
    console.log(JSON.stringify(result, null, 2));
    if (!result.ok) process.exitCode = 1;
    return;
  }
  if (args[0] === "claim") {
    const { coordinationClaim } = await import("./api.mjs");
    const result = await coordinationClaim({
      ...parseApiArgs(args.slice(1), { positionalLane: true }),
      ...(hub ? { hub } : {}),
      ...(stateRoot ? { stateRoot } : {}),
    });
    console.log(JSON.stringify(result, null, 2));
    if (!result.ok) process.exitCode = 1;
    return;
  }
  if (args[0] === "release") {
    const { coordinationRelease } = await import("./api.mjs");
    const result = await coordinationRelease({
      ...parseApiArgs(args.slice(1), { positionalLane: true }),
      ...(hub ? { hub } : {}),
      ...(stateRoot ? { stateRoot } : {}),
    });
    console.log(JSON.stringify(result, null, 2));
    return;
  }
  if (args[0] === "repair") {
    const { coordinationRepair } = await import("./api.mjs");
    const result = await coordinationRepair({
      ...parseRepairArgs(args.slice(1)),
      ...(hub ? { hub } : {}),
      ...(stateRoot ? { stateRoot } : {}),
    });
    console.log(JSON.stringify(result, null, 2));
    return;
  }
  const previousArgv = process.argv;
  const previousHub = process.env.OCENTRA_COORDINATION_HUB;
  const previousLedgerRoot = process.env.LEDGER_ROOT;
  if (hub) process.env.OCENTRA_COORDINATION_HUB = hub;
  if (stateRoot) process.env.LEDGER_ROOT = stateRoot;
  process.argv = [process.execPath, VENDOR_CLI, ...args];
  try {
    await import(`${pathToImportSpecifier(VENDOR_CLI)}?run=${Date.now()}`);
  } finally {
    process.argv = previousArgv;
    if (previousHub === undefined) delete process.env.OCENTRA_COORDINATION_HUB;
    else process.env.OCENTRA_COORDINATION_HUB = previousHub;
    if (previousLedgerRoot === undefined) delete process.env.LEDGER_ROOT;
    else process.env.LEDGER_ROOT = previousLedgerRoot;
  }
}

function parseGlobalCoordinationArgs(rawArgs) {
  const args = [];
  let hub = null;
  let stateRoot = null;
  for (let index = 0; index < rawArgs.length; index += 1) {
    const arg = rawArgs[index];
    if (arg === "--hub") {
      hub = rawArgs[index + 1] ?? null;
      index += 1;
      continue;
    }
    if (arg === "--state-root" || arg === "--stateRoot") {
      stateRoot = rawArgs[index + 1] ?? null;
      index += 1;
      continue;
    }
    args.push(arg);
  }
  return { args: normalizePublicCommandArgs(args), hub, stateRoot };
}

function pathToImportSpecifier(filePath) {
  return `file:///${path.resolve(filePath).replaceAll("\\", "/")}`;
}

function normalizePublicCommandArgs(args) {
  const [command, ...rest] = args;
  if (command === "claim") {
    return args;
  }
  if (command === "release") {
    return args;
  }
  if (command === "guard") {
    const paths = pathValues(rest);
    if (paths.length > 0) {
      return ["guard", ...withoutPathOptions(rest), "--changed", paths.join(",")];
    }
  }
  return args;
}

function pathValues(args) {
  const values = [];
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--paths" || arg === "--path" || arg === "--changed-paths") {
      const value = args[index + 1];
      if (value) values.push(...splitPathList(value));
      index += 1;
    }
  }
  return values;
}

function withoutPathOptions(args) {
  const filtered = [];
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--paths" || arg === "--path" || arg === "--changed-paths") {
      index += 1;
      continue;
    }
    filtered.push(arg);
  }
  return filtered;
}

function optionValue(args, name) {
  const index = args.indexOf(name);
  const value = index >= 0 ? args[index + 1] : undefined;
  return value === undefined || value.startsWith("--") ? null : value;
}

function splitPathList(value) {
  return String(value)
    .split(/[,\n]/u)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function required(value, name) {
  if (!value) throw new Error(`coordination ${name} is required`);
  return value;
}

function parseRepairArgs(args) {
  const action = args.find((arg) => !arg.startsWith("--")) ?? "legacy-hash";
  const limit = optionValue(args, "--limit");
  return {
    action,
    root: optionValue(args, "--root") ?? undefined,
    lane: optionValue(args, "--lane") ?? undefined,
    paths: pathValues(args),
    owner: optionValue(args, "--owner") ?? undefined,
    reason: optionValue(args, "--reason") ?? undefined,
    write: args.includes("--write"),
    dryRun: args.includes("--dry-run") ? true : undefined,
    ...(limit ? { limit: Number(limit) } : {}),
  };
}

function parseApiArgs(args, options = {}) {
  const limit = optionValue(args, "--limit");
  const positionals = positionalArgs(args, [
    "--root",
    "--repo-root",
    "--worktree-root",
    "--cwd",
    "--lane",
    "--paths",
    "--path",
    "--changed",
    "--changed-paths",
    "--session",
    "--session-id",
    "--limit",
    "--reason",
    "--operation",
    "--lock-kind",
    "--lockKind",
    "--on-conflict",
    "--onConflict",
    "--claim-group",
    "--claimGroup",
    "--wait-ms",
    "--waitMs",
    "--project-id",
    "--projectId",
    "--git-remote",
    "--gitRemote",
    "--branch",
    "--commit",
    "--codex-thread-id",
    "--codexThreadId",
    "--codex-session-id",
    "--codexSessionId",
  ]);
  const positionalLane = options.positionalLane && positionals[0] ? positionals[0] : undefined;
  const positionalPaths = options.positionalLane ? positionals.slice(1) : positionals;
  return {
    root: optionValue(args, "--root") ?? undefined,
    repoRoot: optionValue(args, "--repo-root") ?? optionValue(args, "--repoRoot") ?? undefined,
    worktreeRoot: optionValue(args, "--worktree-root") ?? optionValue(args, "--worktreeRoot") ?? undefined,
    cwd: optionValue(args, "--cwd") ?? undefined,
    lane: optionValue(args, "--lane") ?? positionalLane,
    paths: [...pathValues(args), ...positionalPaths],
    changedPaths: pathValuesFor(args, "--changed", "--changed-paths"),
    reason: optionValue(args, "--reason") ?? undefined,
    operation: optionValue(args, "--operation") ?? undefined,
    lockKind: optionValue(args, "--lock-kind") ?? optionValue(args, "--lockKind") ?? undefined,
    onConflict: optionValue(args, "--on-conflict") ?? optionValue(args, "--onConflict") ?? undefined,
    claimGroup: optionValue(args, "--claim-group") ?? optionValue(args, "--claimGroup") ?? undefined,
    waitMs: numberOption(args, "--wait-ms", "--waitMs"),
    projectId: optionValue(args, "--project-id") ?? optionValue(args, "--projectId") ?? undefined,
    gitRemote: optionValue(args, "--git-remote") ?? optionValue(args, "--gitRemote") ?? undefined,
    branch: optionValue(args, "--branch") ?? undefined,
    commit: optionValue(args, "--commit") ?? undefined,
    codexThreadId: optionValue(args, "--codex-thread-id") ?? optionValue(args, "--codexThreadId") ?? undefined,
    codexSessionId: optionValue(args, "--codex-session-id") ?? optionValue(args, "--codexSessionId") ?? undefined,
    sessionId: optionValue(args, "--session") ?? optionValue(args, "--session-id") ?? undefined,
    allowPrimaryWithoutClaims: args.includes("--allow-primary-without-claims"),
    allowMergeRisks: args.includes("--allow-merge-risks"),
    focused: args.includes("--unfocused") || args.includes("--all-conflicts") ? false : true,
    ...(limit ? { limit: Number(limit) } : {}),
  };
}

function positionalArgs(args, valueOptions) {
  const values = [];
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (valueOptions.includes(arg)) {
      index += 1;
      continue;
    }
    if (arg.startsWith("--")) continue;
    values.push(arg);
  }
  return values;
}

function numberOption(args, ...names) {
  for (const name of names) {
    const value = optionValue(args, name);
    if (value !== null) return Number(value);
  }
  return undefined;
}

function pathValuesFor(args, ...names) {
  const values = [];
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (names.includes(arg)) {
      const value = args[index + 1];
      if (value) values.push(...splitPathList(value));
      index += 1;
    }
  }
  return values;
}
