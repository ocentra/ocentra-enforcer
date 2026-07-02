#!/usr/bin/env node

import {
  pushOption,
} from "./rust-rules-mcp-fallback-options.mjs";
import { coordinationCommandFallbackArgs } from "./rust-rules-mcp-fallback-command-builders.mjs";

const DIRECT_COMMANDS = new Map([
  ["ocentra_enforcer_coordination_init", "init"],
  ["ocentra_enforcer_coordination_claim", "claim"],
  ["ocentra_enforcer_coordination_release", "release"],
  ["ocentra_enforcer_coordination_closeout", "closeout"],
  ["ocentra_enforcer_coordination_report", "report"],
  ["ocentra_enforcer_coordination_message", "message"],
  ["ocentra_enforcer_coordination_sync", "sync"],
  ["ocentra_enforcer_coordination_ensure", "ensure"],
  ["ocentra_enforcer_coordination_compact", "compact"],
  ["ocentra_enforcer_coordination_repair", "repair"],
]);

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
  if (DIRECT_COMMANDS.has(name)) return DIRECT_COMMANDS.get(name);
  if (name === "ocentra_enforcer_coordination_mail") {
    return mailFallbackCommand(args);
  }
  if (name === "ocentra_enforcer_coordination_peer") {
    return peerFallbackCommand(args);
  }
  return null;
}

function mailFallbackCommand(args) {
  const action = String(args.action ?? "").toLowerCase();
  if (action === "send") return "message";
  if (action === "ack") return "ack";
  return null;
}

function peerFallbackCommand(args) {
  const action = String(args.action ?? "").toLowerCase();
  return ["add", "remove", "sync"].includes(action) ? "peer" : null;
}

function coordinationGlobalFallbackArgs(args) {
  const result = [];
  pushOption(result, "--state-root", args.stateRoot);
  pushOption(result, "--hub", args.hub);
  return result;
}

export {
  coordinationCommandFallbackArgs,
  coordinationFallbackArgs,
  coordinationFallbackCommand,
  coordinationGlobalFallbackArgs,
};
