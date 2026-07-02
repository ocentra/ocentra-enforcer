#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { createHash } from "node:crypto";
import {
  CLI_PATH,
  COORDINATION_WRITE_TOOLS,
  SERVER_ROOT,
} from "./rust-rules-mcp-context.mjs";

function shouldBlockStaleMcpTool(name, args, freshness) {
  if (freshness.directWritesAllowed === true) return false;
  if (COORDINATION_WRITE_TOOLS.has(name)) return true;
  if (name === "ocentra_enforcer_coordination_mail") {
    return ["send", "ack"].includes(String(args.action ?? "").toLowerCase());
  }
  if (name === "ocentra_enforcer_coordination_peer") {
    return ["add", "remove", "sync"].includes(
      String(args.action ?? "").toLowerCase(),
    );
  }
  if (name === "ocentra_enforcer_coordination_repair") {
    return args.write === true || args.dryRun === false;
  }
  return false;
}

function mcpStaleError(name, freshness, args = {}) {
  const reason =
    freshness.hashCompatible === false
      ? "coordination hash compatibility failed"
      : "MCP server is stale";
  const fallback = buildStaleFallback(name, args);
  return {
    ok: false,
    error: `${reason}; refusing ${name} because it may write incompatible coordination events.`,
    operation: name,
    directWritesAllowed: false,
    writeCapable: false,
    fallbackAvailable: fallback !== null,
    reloadRequired: true,
    fallback,
    nextStep: fallback
      ? `Restart Codex Desktop/MCP, or call ${fallback.recommendedTool} with fallback.enforcerRunArguments.`
      : "Restart Codex Desktop/MCP, or use ocentra_enforcer_run to invoke the updated CLI from the pack root.",
    mcpFreshness: freshness,
  };
}

function buildStaleFallback(name, args = {}) {
  const cliArgs = coordinationFallbackArgs(name, args);
  if (cliArgs.length === 0) return null;
  const command = [process.execPath, CLI_PATH, ...cliArgs];
  return {
    recommendedTool: "ocentra_enforcer_run",
    cwd: SERVER_ROOT,
    command,
    commandLine: command.map(quoteCommandArg).join(" "),
    enforcerRunArguments: {
      root: SERVER_ROOT,
      tool: "ocentra-enforcer-coordination-fallback",
      command,
    },
  };
}

function coordinationFallbackArgs(name, args) {
  const command = coordinationFallbackCommand(name, args);
  if (command === null) return [];
  return [
    "coordination",
    command,
    ...coordinationGlobalFallbackArgs(args),
    ...coordinationCommandFallbackArgs(command, args),
    "--json",
  ];
}

function coordinationFallbackCommand(name, args) {
  if (name === "ocentra_enforcer_coordination_init") return "init";
  if (name === "ocentra_enforcer_coordination_claim") return "claim";
  if (name === "ocentra_enforcer_coordination_release") return "release";
  if (name === "ocentra_enforcer_coordination_closeout") return "closeout";
  if (name === "ocentra_enforcer_coordination_report") return "report";
  if (name === "ocentra_enforcer_coordination_message") return "message";
  if (name === "ocentra_enforcer_coordination_sync") return "sync";
  if (name === "ocentra_enforcer_coordination_ensure") return "ensure";
  if (name === "ocentra_enforcer_coordination_compact") return "compact";
  if (name === "ocentra_enforcer_coordination_repair") return "repair";
  if (name === "ocentra_enforcer_coordination_mail") {
    const action = String(args.action ?? "").toLowerCase();
    if (action === "send") return "message";
    if (action === "ack") return "ack";
  }
  if (name === "ocentra_enforcer_coordination_peer") {
    const action = String(args.action ?? "").toLowerCase();
    if (["add", "remove", "sync"].includes(action)) return "peer";
  }
  return null;
}

function coordinationGlobalFallbackArgs(args) {
  const result = [];
  pushOption(result, "--state-root", args.stateRoot);
  pushOption(result, "--hub", args.hub);
  return result;
}

function coordinationCommandFallbackArgs(command, args) {
  if (command === "init") {
    return [
      ...(args.hub ? [String(args.hub)] : []),
      ...commonCoordinationOptions(args, { includePaths: false }),
    ];
  }
  if (command === "claim" || command === "release") {
    return [
      ...commonCoordinationOptions(args, { includePaths: false }),
      ...pathOption("--paths", args.paths),
    ];
  }
  if (command === "closeout") {
    const closeoutArgs = commonCoordinationOptions(args, { includePaths: false });
    pushOption(closeoutArgs, "--owner", args.owner);
    if (args.allOwned === true) closeoutArgs.push("--all-owned");
    if (args.allLanes === true) closeoutArgs.push("--all-lanes");
    if (args.allowOtherNode === true) closeoutArgs.push("--allow-other-node");
    if (args.releaseOwned === false) closeoutArgs.push("--no-release");
    if (args.repairStale === false) closeoutArgs.push("--no-repair-stale");
    return closeoutArgs;
  }
  if (command === "guard") {
    return [
      ...commonCoordinationOptions(args, { includePaths: false }),
      ...pathOption("--paths", args.paths ?? args.changedPaths),
    ];
  }
  if (command === "repair") {
    const repairArgs = [String(args.action ?? "legacy-hash")];
    repairArgs.push(...commonCoordinationOptions(args));
    pushOption(repairArgs, "--owner", args.owner);
    if (args.write === true) repairArgs.push("--write");
    if (args.dryRun === true) repairArgs.push("--dry-run");
    return repairArgs;
  }
  if (command === "message" || command === "msg") {
    const to = args.to ?? args.lane;
    const body = args.body ?? args.message ?? args.summary ?? args.subject;
    const messageArgs = commonCoordinationOptions(args, {
      includeLane: false,
      includePaths: false,
    });
    pushOption(messageArgs, "--from", args.from);
    pushOption(messageArgs, "--to", to);
    pushOption(messageArgs, "--subject", args.subject);
    pushOption(messageArgs, "--body", body);
    return messageArgs;
  }
  if (command === "ack") {
    return [
      ...commonCoordinationOptions(args, { includePaths: false }),
      ...(args.messageId ? [String(args.messageId)] : []),
      ...(args.id ? [String(args.id)] : []),
    ];
  }
  return commonCoordinationOptions(args);
}

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

function buildMcpFingerprint(mcpFingerprintFiles) {
  const files = mcpFingerprintFiles.map(fingerprintFile);
  const digest = createHash("sha256")
    .update(
      JSON.stringify(
        files.map((file) => ({
          path: file.path,
          exists: file.exists,
          sha256: file.sha256,
          byteLength: file.byteLength,
        })),
      ),
    )
    .digest("hex");
  return {
    digest,
    packageVersion: readPackageVersion(),
    files,
  };
}

function fingerprintFile(filePath) {
  const label = normalizeFingerprintLabel(filePath);
  const resolved = resolveFingerprintFile(filePath);
  if (!fs.existsSync(resolved)) {
    return {
      path: label,
      resolvedPath: resolved,
      exists: false,
      sha256: null,
      byteLength: 0,
      mtimeMs: null,
    };
  }
  const buffer = fs.readFileSync(resolved);
  const stat = fs.statSync(resolved);
  return {
    path: label,
    resolvedPath: resolved,
    exists: true,
    sha256: createHash("sha256").update(buffer).digest("hex"),
    byteLength: buffer.length,
    mtimeMs: stat.mtimeMs,
  };
}

function changedFingerprintFiles(startupFiles, currentFiles) {
  const startupByPath = new Map(startupFiles.map((file) => [file.path, file]));
  const currentByPath = new Map(currentFiles.map((file) => [file.path, file]));
  const paths = [...new Set([...startupByPath.keys(), ...currentByPath.keys()])].sort();
  return paths
    .map((filePath) => {
      const startup = startupByPath.get(filePath);
      const current = currentByPath.get(filePath);
      const changed =
        startup?.exists !== current?.exists ||
        startup?.sha256 !== current?.sha256 ||
        startup?.byteLength !== current?.byteLength;
      return changed
        ? {
            path: filePath,
            startup: summarizeFingerprintEntry(startup),
            current: summarizeFingerprintEntry(current),
          }
        : null;
    })
    .filter(Boolean);
}

function summarizeFingerprintEntry(entry) {
  return entry
    ? {
        exists: entry.exists,
        sha256: entry.sha256,
        byteLength: entry.byteLength,
      }
    : null;
}

function readPackageVersion() {
  try {
    return JSON.parse(
      fs.readFileSync(path.join(SERVER_ROOT, "package.json"), "utf8"),
    ).version;
  } catch {
    return null;
  }
}

function extraFingerprintFiles() {
  return String(process.env.OCENTRA_ENFORCER_MCP_FINGERPRINT_EXTRA ?? "")
    .split(path.delimiter)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function resolveFingerprintFile(filePath) {
  return path.isAbsolute(filePath)
    ? path.resolve(filePath)
    : path.join(SERVER_ROOT, filePath);
}

function normalizeFingerprintLabel(filePath) {
  return path.isAbsolute(filePath)
    ? path.resolve(filePath).replaceAll("\\", "/")
    : filePath.replaceAll("\\", "/");
}

function normalizeToolName(name) {
  return String(name ?? "").replace(/^rust_rules_/u, "ocentra_enforcer_");
}

export {
  buildMcpFingerprint,
  changedFingerprintFiles,
  commonCoordinationOptions,
  coordinationCommandFallbackArgs,
  coordinationFallbackArgs,
  coordinationFallbackCommand,
  coordinationGlobalFallbackArgs,
  extraFingerprintFiles,
  fingerprintFile,
  mcpStaleError,
  normalizeFingerprintLabel,
  normalizeToolName,
  pathOption,
  pushOption,
  quoteCommandArg,
  readPackageVersion,
  reasonOption,
  resolveFingerprintFile,
  shouldBlockStaleMcpTool,
  stringArray,
  summarizeFingerprintEntry,
};
