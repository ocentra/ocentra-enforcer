#!/usr/bin/env node

function commonCoordinationOptions(args, options = {}) {
  const result = [];
  if (options.includeLane !== false) pushOption(result, "--lane", args.lane);
  pushOption(result, "--root", args.root);
  pushOption(result, "--repo-root", args.repoRoot);
  pushOption(result, "--worktree-root", args.worktreeRoot);
  pushOption(result, "--cwd", args.cwd);
  pushOption(result, "--project-id", args.projectId);
  pushOption(result, "--git-remote", args.gitRemote);
  pushOption(result, "--branch", args.branch);
  pushOption(result, "--commit", args.commit);
  pushOption(result, "--codex-thread-id", args.codexThreadId);
  pushOption(result, "--codex-session-id", args.codexSessionId);
  pushOption(result, "--session-id", args.sessionId);
  pushOption(result, "--operation", args.operation);
  pushOption(result, "--lock-kind", args.lockKind);
  pushOption(result, "--on-conflict", args.onConflict);
  pushOption(result, "--claim-group", args.claimGroup);
  pushOption(result, "--wait-ms", args.waitMs);
  pushOption(result, "--limit", args.limit);
  if (options.includePaths !== false) {
    result.push(...pathOption("--paths", args.paths ?? args.changedPaths));
  }
  result.push(...reasonOption(args));
  return result;
}

function reasonOption(args) {
  return args.reason ? ["--reason", String(args.reason)] : [];
}

function pathOption(name, value) {
  const paths = stringArray(value);
  return paths.length > 0 ? [name, paths.join(",")] : [];
}

function pushOption(result, name, value) {
  if (value !== undefined && value !== null && value !== "") {
    result.push(name, String(value));
  }
}

function stringArray(value) {
  if (Array.isArray(value)) return value.map(String).filter(Boolean);
  if (value === undefined || value === null || value === "") return [];
  return String(value)
    .split(/[,\\n]/u)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function quoteCommandArg(value) {
  const text = String(value);
  if (/^[A-Za-z0-9_./:=+-]+$/u.test(text)) return text;
  return `"${text.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;
}

export {
  commonCoordinationOptions,
  pathOption,
  pushOption,
  quoteCommandArg,
  reasonOption,
  stringArray,
};
