import { normalizeCheckName } from "../src/checks.mjs";
import { normalizeVerifyMode } from "./rust-rules-scan-core-args-options.mjs";

const COMMANDS = new Set([
  "init",
  "route",
  "check",
  "advise",
  "verify",
  "scan",
  "cargo",
  "doctor",
  "explain",
  "run",
  "runs",
  "codex",
]);

function takeOptionalToken(tokens) {
  return tokens[0] && !tokens[0].startsWith("-") ? tokens.shift() : null;
}

function handleCodexCommand(args, tokens) {
  const codexCommand = takeOptionalToken(tokens) ?? "install";
  if (codexCommand === "install") {
    args.command = "codex-install";
    args.adapters = ["codex", "mcp"];
    return;
  }
  if (codexCommand === "uninstall") {
    args.command = "codex-uninstall";
    return;
  }
  if (codexCommand === "doctor") {
    args.command = "codex-doctor";
    return;
  }
  throw new Error(`Unknown codex command: ${codexCommand}`);
}

const COMMAND_HEAD_HANDLERS = {
  explain: (args, tokens) => {
    args.explainRuleId = tokens.shift() ?? null;
  },
  advise: (args, tokens) => {
    args.adviseTarget = takeOptionalToken(tokens);
  },
  check: (args, tokens) => {
    const checkName = takeOptionalToken(tokens);
    args.checkName = checkName ? normalizeCheckName(checkName) : null;
  },
  route: (args, tokens) => {
    if (tokens[0] && !tokens[0].startsWith("-")) args.routeRuleId = tokens.shift();
  },
  verify: (args, tokens) => {
    if (tokens[0] && !tokens[0].startsWith("-")) args.verifyMode = normalizeVerifyMode(tokens.shift());
  },
  runs: (args, tokens) => {
    args.runsCommand = takeOptionalToken(tokens) ?? "list";
  },
  codex: handleCodexCommand,
};

export function consumeCommand(tokens) {
  if (!tokens[0] || !COMMANDS.has(tokens[0])) return "scan";
  return tokens.shift();
}

export function applyCommandHead(args, tokens) {
  COMMAND_HEAD_HANDLERS[args.command]?.(args, tokens);
}
