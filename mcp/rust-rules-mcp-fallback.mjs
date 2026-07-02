#!/usr/bin/env node

import process from "node:process";
import {
  CLI_PATH,
  COORDINATION_WRITE_TOOLS,
  SERVER_ROOT,
} from "./rust-rules-mcp-context.mjs";
import {
  coordinationCommandFallbackArgs,
  coordinationFallbackArgs,
  coordinationFallbackCommand,
  coordinationGlobalFallbackArgs,
} from "./rust-rules-mcp-fallback-routing.mjs";
import {
  commonCoordinationOptions,
  pathOption,
  pushOption,
  quoteCommandArg,
  reasonOption,
  stringArray,
} from "./rust-rules-mcp-fallback-options.mjs";

const WRITE_ACTIONS_BY_TOOL = new Map([
  ["ocentra_enforcer_coordination_mail", ["send", "ack"]],
  ["ocentra_enforcer_coordination_peer", ["add", "remove", "sync"]],
]);

function shouldBlockStaleMcpTool(name, args, freshness) {
  if (freshness.directWritesAllowed === true) return false;
  if (COORDINATION_WRITE_TOOLS.has(name)) return true;
  if (name === "ocentra_enforcer_coordination_repair") {
    return args.write === true || args.dryRun === false;
  }
  const writeActions = WRITE_ACTIONS_BY_TOOL.get(name);
  if (!writeActions) return false;
  return writeActions.includes(String(args.action ?? "").toLowerCase());
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

export {
  commonCoordinationOptions,
  coordinationCommandFallbackArgs,
  coordinationFallbackArgs,
  coordinationFallbackCommand,
  coordinationGlobalFallbackArgs,
  mcpStaleError,
  pathOption,
  pushOption,
  quoteCommandArg,
  reasonOption,
  shouldBlockStaleMcpTool,
  stringArray,
};
