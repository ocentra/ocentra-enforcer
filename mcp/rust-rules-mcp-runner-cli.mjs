import fs from "node:fs";
import path from "node:path";
import { CLI_PATH, SERVER_ROOT } from "./rust-rules-mcp-helpers.mjs";
import { appendScopeArgs } from "./rust-rules-mcp-runner-scope.mjs";

export function buildCliInvocation(command, args) {
  const root = path.resolve(args.root ?? process.cwd());
  const cliArgs = [CLI_PATH, command];
  appendCommandArgs(cliArgs, command, args);
  cliArgs.push("--root", root, "--json");
  appendOption(cliArgs, "--config", resolveConfigPath(root, args));
  appendJoinedList(cliArgs, "--languages", args.languages);
  appendOption(cliArgs, "--check-config", args.checkConfigPath);
  appendOption(cliArgs, "--output", args.output);
  appendFlag(cliArgs, "--dry-run", args.dryRun);
  appendFlag(cliArgs, "--staged", args.staged);
  appendFlag(cliArgs, "--tracked", args.tracked);
  appendFlag(cliArgs, "--strict-empty-test-trees", args.strictEmptyTestTrees);
  appendScopeArgs(cliArgs, args);
  return { cliArgs, root };
}

function appendCommandArgs(cliArgs, command, args) {
  if (command === "check") {
    cliArgs.push(args.check);
  }
}

function resolveConfigPath(root, args) {
  if (args.configPath) {
    return path.isAbsolute(args.configPath)
      ? args.configPath
      : path.join(root, args.configPath);
  }
  const profile = args.profile ?? null;
  if (profile === null || profile === "" || profile === "strict") {
    return null;
  }
  if (!/^[A-Za-z0-9][A-Za-z0-9_.-]*$/u.test(profile)) {
    throw new Error(`Invalid profile name: ${profile}`);
  }
  const profilePath = path.join(SERVER_ROOT, "profiles", `${profile}.json`);
  if (!pathExists(profilePath)) {
    throw new Error(
      `Unknown Ocentra Enforcer profile "${profile}". Expected ${profilePath}.`,
    );
  }
  return profilePath;
}

function appendOption(cliArgs, flag, value) {
  if (value) {
    cliArgs.push(flag, value);
  }
}

function appendFlag(cliArgs, flag, enabled) {
  if (enabled) {
    cliArgs.push(flag);
  }
}

function appendJoinedList(cliArgs, flag, values) {
  if (Array.isArray(values) && values.length > 0) {
    cliArgs.push(flag, values.join(","));
  }
}

function pathExists(target) {
  return Boolean(target) && fs.existsSync(target);
}
