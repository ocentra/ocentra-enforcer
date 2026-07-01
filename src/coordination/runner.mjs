import path from "node:path";
import process from "node:process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const VENDOR_CLI = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "vendor",
  "cli.js",
);

export async function runCoordinationCli(rawArgs = process.argv.slice(2)) {
  const { args, hub, stateRoot } = parseGlobalCoordinationArgs(rawArgs);
  if (isCompatCommand(args[0])) {
    await runCompatCommand(args[0], args.slice(1), { hub, stateRoot });
    return;
  }
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
  if (args[0] === "closeout") {
    const { coordinationCloseout } = await import("./api.mjs");
    const result = await coordinationCloseout({
      ...parseCloseoutArgs(args.slice(1)),
      ...(hub ? { hub } : {}),
      ...(stateRoot ? { stateRoot } : {}),
    });
    console.log(JSON.stringify(result, null, 2));
    if (!result.ok) process.exitCode = 1;
    return;
  }
  if (args[0] === "message" || args[0] === "msg") {
    const { coordinationMessage } = await import("./api.mjs");
    const result = await coordinationMessage({
      ...parseMessageArgs(args.slice(1)),
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

function isCompatCommand(command) {
  return (
    typeof command === "string" &&
    (command.startsWith("hub:") ||
      command.startsWith("lanes:") ||
      command.startsWith("ledger:"))
  );
}

async function runCompatCommand(command, rawArgs, context) {
  const options = parseCompatOptions(rawArgs);
  const lane = compatLane(options);
  switch (command) {
    case "lanes:init":
      await runNested(["init", context.hub ?? "ocentra-parent", "--lane", lane], context);
      return;
    case "lanes:status":
    case "hub:status":
    case "ledger:doctor":
      await runNested(["doctor"], context);
      return;
    case "ledger:root":
    case "ledger:install":
    case "ledger:build":
      await runNested(["root"], context);
      return;
    case "ledger:ensure":
    case "ledger:dashboard":
      await runNested(["ensure"], context);
      return;
    case "ledger:inbox":
    case "hub:inbox":
      await runNested(["inbox", lane], context);
      return;
    case "ledger:workers":
    case "hub:heartbeats":
      await runNested(["workers"], context);
      return;
    case "ledger:free":
      await runNested(["workers", "free"], context);
      return;
    case "ledger:tasks":
      await runNested(["tasks", "active"], context);
      return;
    case "ledger:message":
      await runNested(["msg", required(options.lane ?? options.to ?? options._[0], "lane"), compatMessageBody(options)], context);
      return;
    case "ledger:notify":
    case "hub:notify":
      await runNested(["notify", "--lane", lane, ...compatForwardedFlags(rawArgs, ["--json", "--peek", "--exit-code"])], context);
      return;
    case "ledger:sync":
    case "hub:state:sync":
      await runNested(["sync", ...rawArgs], context);
      return;
    case "ledger:guard":
    case "lanes:guard":
    case "hub:guard":
      await runNested(["guard", "--lane", lane, ...compatChangedArgs(options)], context);
      return;
    case "lanes:claim":
      await runNested(["worker", required(options.lane ?? lane, "lane"), "started", required(options.task, "task")], context);
      return;
    case "lanes:free":
      await runNested(["worker", lane, "idle", options["next-action"] ?? "lane released"], context);
      return;
    case "hub:message":
      await runNested(["message", "--to", required(options.lane ?? options.to ?? options._[0], "lane"), "--body", compatMessageBody(options)], context);
      return;
    case "hub:ack":
      await ackLatestOrExplicit(lane, options, context);
      return;
    case "hub:heartbeat":
      await runNested(["heartbeat", lane, mapHeartbeatState(options.state), options.note ?? options.summary ?? "heartbeat"], context);
      return;
    case "hub:report":
      await reportWithPrimaryNotification(lane, options, context);
      return;
    case "hub:lock":
      await runNested(["claim", "--lane", lane, "--paths", required(options.paths, "paths"), "--reason", options.reason ?? "claimed from coordination alias"], context);
      return;
    case "hub:unlock":
      await runNested(["release", "--lane", lane, "--paths", required(options.paths, "paths"), "--reason", options.reason ?? "released from coordination alias"], context);
      return;
    case "hub:watch":
      await runNested(["inbox", lane], context);
      return;
    case "hub:hook":
      await printHookContext(lane, context);
      return;
    case "hub:thread-mode":
      printThreadModeStatus(lane, context);
      return;
    case "hub:thread:upgrade":
      setThreadMode(lane, requiredCurrentPromptedThread(options, context), "manual-only", context);
      return;
    case "hub:thread:default":
      setThreadMode(lane, requiredCurrentPromptedThread(options, context), "default", context);
      return;
    case "hub:delegate:grant":
      setDelegateGrant(lane, requiredCurrentPromptedThread(options, context), required(options["session-id"], "session-id"), context);
      return;
    case "hub:delegate:revoke":
      clearDelegateGrant(lane, requiredCurrentPromptedThread(options, context), required(options["session-id"], "session-id"), context);
      return;
    case "hub:lane-ledger:audit":
      console.log("Ocentra Enforcer keeps live coordination state outside product repos; no product-repo ledger sync is needed.");
      return;
    default:
      throw new Error(`unknown coordination compatibility command: ${command}`);
  }
}

async function runNested(args, context) {
  await runCoordinationCli([
    ...(context.stateRoot ? ["--state-root", context.stateRoot] : []),
    ...(context.hub ? ["--hub", context.hub] : []),
    ...args,
  ]);
}

function compatLane(options) {
  return (
    options.lane ??
    process.env.LEDGER_LANE ??
    process.env.OCENTRA_COORDINATION_LANE ??
    process.env.OCENTRA_PARENT_LEDGER_LANE ??
    inferLane(process.cwd())
  );
}

function inferLane(rootPath) {
  const normalized = String(rootPath).replace(/\\/gu, "/");
  const match = normalized.match(/(?:^|[/_-])((?:codex-[a-z])|(?:E-[A-Z]))(?:$|[/_-])/u);
  return match?.[1] ?? "primary";
}

function parseCompatOptions(args) {
  const parsed = { _: [] };
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--") continue;
    if (!arg.startsWith("--")) {
      parsed._.push(arg);
      continue;
    }
    const equalsIndex = arg.indexOf("=");
    if (equalsIndex > 2) {
      parsed[arg.slice(2, equalsIndex)] = arg.slice(equalsIndex + 1);
      continue;
    }
    const key = arg.slice(2);
    const next = args[index + 1];
    if (next === undefined || next.startsWith("--")) {
      parsed[key] = true;
      continue;
    }
    parsed[key] = next;
    index += 1;
  }
  return parsed;
}

function compatMessageBody(options) {
  const subject = options.subject ?? "Ledger message";
  const body = options.body ?? options.message ?? options._.slice(options.lane || options.to ? 0 : 1).join(" ");
  return `${subject}\n\n${body}`.trim();
}

function compatChangedArgs(options) {
  const changed = options.paths ?? options.path ?? options.changed ?? options["changed-paths"];
  return changed ? ["--changed", changed] : [];
}

function compatForwardedFlags(args, names) {
  return args.filter((arg) => names.includes(arg));
}

async function ackLatestOrExplicit(lane, options, context) {
  const explicit = options["message-id"] ?? options.messageId ?? options._[0];
  if (explicit) {
    await runNested(["ack", "--lane", lane, explicit], context);
    return;
  }
  const { coordinationInbox, coordinationAck } = await import("./api.mjs");
  const inbox = await coordinationInbox({
    lane,
    ...(context.hub ? { hub: context.hub } : {}),
    ...(context.stateRoot ? { stateRoot: context.stateRoot } : {}),
  });
  const latest = Array.isArray(inbox.inbox) ? inbox.inbox.at(-1) : undefined;
  if (!latest?.id) {
    console.log(`No unread coordination messages for ${lane}.`);
    return;
  }
  const result = await coordinationAck({
    lane,
    messageId: latest.id,
    ...(context.hub ? { hub: context.hub } : {}),
    ...(context.stateRoot ? { stateRoot: context.stateRoot } : {}),
  });
  console.log(JSON.stringify(result, null, 2));
}

async function reportWithPrimaryNotification(lane, options, context) {
  const body = reportBody(options);
  await runNested(["report", "--lane", lane, body], context);
  if (lane !== "primary" && options["no-primary-notify"] !== true && /^(?:PR[-_ ]?READY|DONE|BLOCKED)\b/iu.test(body.trim())) {
    await runNested(["msg", "primary", `Worker report from ${lane}: ${body.split(/\r?\n/u)[0]}\n\n${body}`], context);
  }
}

function reportBody(options) {
  const summary = options.summary ?? options._.join(" ").trim();
  const details = options.details;
  validateLifecycleReport(summary, details);
  return details === undefined ? summary : `${summary}\n\n${details}`;
}

function validateLifecycleReport(summary, details) {
  const kind = lifecycleReportKind(summary);
  if (kind === undefined) return;
  if (details === undefined || details.trim().length === 0) {
    throw new Error(`${kind} reports require a structured --details block with lane, threadId, assignedBy, plan, workpack, worktree, branch, and scope.`);
  }
  const fields = parseMetadataFields(details);
  const requiredFields = ["lane", "threadid", "assignedby", "plan", "workpack", "worktree", "branch", "scope"];
  const stateRequired = {
    STARTED: ["startedat"],
    BLOCKED: ["blocker"],
    PR_READY: ["validation"],
    DONE: ["validation", "commit"],
  }[kind] ?? [];
  const missing = [...requiredFields, ...stateRequired].filter((field) => !((fields.get(field) ?? "").trim()));
  if (missing.length > 0) {
    throw new Error(`${kind} reports require structured fields: ${missing.join(", ")}. Use key: value lines in --details.`);
  }
}

function lifecycleReportKind(summary) {
  const firstLine = summary.split(/\r?\n/u)[0]?.trim() ?? "";
  const match = firstLine.match(/^(STARTED|BLOCKED|PR(?:[_ -]?READY)|DONE)\b/iu);
  if (match === null) return undefined;
  const token = match[1].replace(/[\s-]/gu, "_").toUpperCase();
  return token === "PR_READY" ? "PR_READY" : token;
}

function parseMetadataFields(details) {
  const fields = new Map();
  for (const rawLine of details.split(/\r?\n/gu)) {
    const line = rawLine.trim().replace(/^[*-]\s+/u, "");
    const match = line.match(/^([A-Za-z][A-Za-z0-9_-]*)\s*[:=]\s*(.+)$/u);
    if (match !== null) fields.set(match[1].toLowerCase(), match[2].trim());
  }
  return fields;
}

async function printHookContext(lane, context) {
  const input = readStdinJson();
  const eventName = typeof input.hook_event_name === "string" ? input.hook_event_name : "hook";
  const sessionId = sanitizeSessionId(input.session_id ?? input.sessionId);
  const state = readThreadState(context);
  state.latestHookSessionId = sessionId ?? state.latestHookSessionId;
  state.latestHookEventName = eventName;
  if (eventName === "UserPromptSubmit" && sessionId) {
    state.latestUserPromptSessionId = sessionId;
    state.latestUserPromptMode =
      state.latestUserPromptSessionId === sessionId && state.latestUserPromptMode === "manual-only"
        ? "manual-only"
        : "default";
  }
  const lines = [
    "Ocentra Enforcer coordination context:",
    `- Hook event: ${eventName}.`,
    "- Current lane is configured for this checkout. Set LEDGER_LANE to override lane identity when needed.",
  ];
  if (lane !== "primary" && sessionId) {
    const claim = await tryClaimSession(lane, sessionId, eventName, context);
    const activeSessionId = claim.activeSessionId ?? readActiveSessionId(context);
    writeThreadState(state, context);
    if (claim.ok) {
      lines.push("- Active Codex session lease is held by this thread; exact-file claims are the write gate.");
    } else if (state.delegateGrants[sessionId] !== undefined) {
      lines.push(`- COORDINATED-DELEGATE-GRANT: writable access delegated by ${state.delegateGrants[sessionId]} without taking the lane lease.`);
    } else {
      lines.push(`- READ-ONLY: this lane is already owned by another active Codex session (${activeSessionId ?? "unknown"}).`);
    }
  } else {
    writeThreadState(state, context);
  }
  lines.push(
    "- State root is external to the product repo. Use the Enforcer coordination root command to inspect it.",
    "- Check work with coordination health, inbox, presence, workers, and tasks.",
    "- Claim exact file paths only with coordination claim or hub:lock; release them immediately after the edit.",
    "- Report STARTED, BLOCKED, PR_READY, DONE, and handoffs through coordination report/message."
  );
  console.log(JSON.stringify({
    hookSpecificOutput: {
      additionalContext: lines.join("\n"),
      hookEventName: normalizeHookEvent(eventName),
    },
  }));
}

async function tryClaimSession(lane, sessionId, eventName, context) {
  const { coordinationRoot } = await import("./api.mjs");
  const { loadIdentity } = await import("./vendor/identity.js");
  const { materialize } = await import("./vendor/materialize.js");
  const { appendEvent } = await import("./vendor/stream.js");
  const root = coordinationRoot(context);
  const config = await loadIdentity(root);
  const state = await materialize(root);
  const existing = state.sessions.get(lane);
  if (existing !== undefined && existing.sessionId !== sessionId) {
    return { ok: false, activeSessionId: existing.sessionId };
  }
  await appendEvent(root, config, lane, {
    type: "session.claim",
    sessionId,
    ttlSeconds: 7200,
    summary: `${eventName} hook active`,
  });
  writeActiveSessionId(sessionId, context);
  return { ok: true, activeSessionId: sessionId };
}

function sanitizeSessionId(value) {
  return typeof value === "string" && value.length > 0 ? value.replace(/[^A-Za-z0-9._-]/gu, "_") : undefined;
}

function requiredCurrentPromptedThread(options, context) {
  if (options["session-id"] !== undefined) return options["session-id"];
  const state = readThreadState(context);
  if (state.latestUserPromptSessionId === null) {
    throw new Error("Thread mode commands require the current thread after a real user prompt.");
  }
  return state.latestUserPromptSessionId;
}

function setDelegateGrant(lane, delegatedBy, targetSessionId, context) {
  const state = readThreadState(context);
  state.delegateGrants[targetSessionId] = delegatedBy;
  writeThreadState(state, context);
  console.log(`delegate-grant-set: lane=${lane} session=${targetSessionId} delegated-by=${delegatedBy}`);
}

function clearDelegateGrant(lane, _delegatedBy, targetSessionId, context) {
  const state = readThreadState(context);
  delete state.delegateGrants[targetSessionId];
  writeThreadState(state, context);
  console.log(`delegate-grant-cleared: lane=${lane} session=${targetSessionId}`);
}

function setThreadMode(lane, sessionId, mode, context) {
  const state = readThreadState(context);
  state.latestUserPromptSessionId = sessionId;
  state.latestUserPromptMode = mode;
  writeThreadState(state, context);
  console.log(`thread-mode-set: lane=${lane} session=${sessionId} mode=${mode}`);
}

function printThreadModeStatus(lane, context) {
  const state = readThreadState(context);
  const activeSessionId = readActiveSessionId(context);
  console.log([
    `thread-mode: lane=${lane}`,
    `active-session=${activeSessionId ?? "none"}`,
    `latest-user-prompt-session=${state.latestUserPromptSessionId ?? "none"}`,
    `latest-user-prompt-mode=${state.latestUserPromptMode}`,
    `write-grants=${Object.entries(state.delegateGrants).length === 0 ? "none" : Object.entries(state.delegateGrants).map(([sessionId, delegatedBy]) => `${sessionId}:${delegatedBy}`).join(",")}`,
  ].join(" "));
}

function threadStatePath(context, fileName) {
  const root = context.stateRoot ?? path.join(process.cwd(), ".ledger");
  return path.join(root, fileName);
}

function readThreadState(context) {
  const filePath = threadStatePath(context, "thread-mode-state.json");
  if (!existsSync(filePath)) {
    return {
      latestHookSessionId: null,
      latestHookEventName: null,
      latestUserPromptSessionId: null,
      latestUserPromptMode: "default",
      delegateGrants: {},
    };
  }
  const parsed = JSON.parse(readFileSync(filePath, "utf8"));
  return {
    latestHookSessionId: typeof parsed.latestHookSessionId === "string" ? parsed.latestHookSessionId : null,
    latestHookEventName: typeof parsed.latestHookEventName === "string" ? parsed.latestHookEventName : null,
    latestUserPromptSessionId: typeof parsed.latestUserPromptSessionId === "string" ? parsed.latestUserPromptSessionId : null,
    latestUserPromptMode: parsed.latestUserPromptMode === "manual-only" ? "manual-only" : "default",
    delegateGrants: parsed.delegateGrants !== null && typeof parsed.delegateGrants === "object" ? parsed.delegateGrants : {},
  };
}

function writeThreadState(state, context) {
  const filePath = threadStatePath(context, "thread-mode-state.json");
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, JSON.stringify(state, null, 2));
}

function readActiveSessionId(context) {
  const filePath = threadStatePath(context, "active-session.json");
  if (!existsSync(filePath)) return null;
  try {
    const parsed = JSON.parse(readFileSync(filePath, "utf8"));
    return typeof parsed.sessionId === "string" ? parsed.sessionId : null;
  } catch {
    return null;
  }
}

function writeActiveSessionId(sessionId, context) {
  const filePath = threadStatePath(context, "active-session.json");
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, JSON.stringify({ sessionId }, null, 2));
}

function readStdinJson() {
  try {
    const text = readFileSync(0, "utf8").trim();
    return text.length === 0 ? {} : JSON.parse(text);
  } catch {
    return {};
  }
}

function normalizeHookEvent(value) {
  return String(value)
    .split(/[_-]/u)
    .map((part) => `${part.slice(0, 1).toUpperCase()}${part.slice(1)}`)
    .join("");
}

function mapHeartbeatState(value) {
  if (value === "alive" || value === "hook" || value === undefined) return "online";
  return value;
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

function parseCloseoutArgs(args) {
  const common = parseApiArgs(args, { positionalLane: true });
  return {
    ...common,
    owner: optionValue(args, "--owner") ?? optionValue(args, "--writer") ?? undefined,
    threadId: optionValue(args, "--thread") ?? optionValue(args, "--thread-id") ?? undefined,
    releaseOwned: args.includes("--no-release") ? false : true,
    repairStale: args.includes("--no-repair-stale") ? false : true,
    allOwned: args.includes("--all-owned"),
    allLanes: args.includes("--all-lanes"),
    allowOtherNode: args.includes("--allow-other-node"),
  };
}

function parseMessageArgs(args) {
  const common = parseApiArgs(args);
  const positionals = common.paths ?? [];
  const to = optionValue(args, "--to") ?? positionals[0] ?? undefined;
  const body =
    optionValue(args, "--body") ??
    optionValue(args, "--message") ??
    (positionals.length > 1 ? positionals.slice(1).join(" ") : undefined);
  const { paths: _paths, changedPaths: _changedPaths, ...rest } = common;
  return {
    ...rest,
    from: optionValue(args, "--from") ?? undefined,
    to,
    body,
    subject: optionValue(args, "--subject") ?? undefined,
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
    "--from",
    "--to",
    "--body",
    "--message",
    "--subject",
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
