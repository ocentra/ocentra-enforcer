#!/usr/bin/env node

import {
  commonCoordinationOptions,
  pathOption,
  pushOption,
} from "./rust-rules-mcp-fallback-options.mjs";

const COORDINATION_COMMAND_BUILDERS = {
  init(args) {
    return [
      ...(args.hub ? [String(args.hub)] : []),
      ...commonCoordinationOptions(args, { includePaths: false }),
    ];
  },
  claim(args) {
    return releaseLikeFallbackArgs(args);
  },
  release(args) {
    return releaseLikeFallbackArgs(args);
  },
  closeout(args) {
    return closeoutFallbackArgs(args);
  },
  guard(args) {
    return [
      ...commonCoordinationOptions(args, { includePaths: false }),
      ...pathOption("--paths", args.paths ?? args.changedPaths),
    ];
  },
  repair(args) {
    return repairFallbackArgs(args);
  },
  message(args) {
    return messageFallbackArgs(args);
  },
  msg(args) {
    return messageFallbackArgs(args);
  },
  ack(args) {
    return ackFallbackArgs(args);
  },
};

function coordinationCommandFallbackArgs(command, args) {
  const builder = COORDINATION_COMMAND_BUILDERS[command];
  return builder ? builder(args) : commonCoordinationOptions(args);
}

function releaseLikeFallbackArgs(args) {
  return [
    ...commonCoordinationOptions(args, { includePaths: false }),
    ...pathOption("--paths", args.paths),
  ];
}

function closeoutFallbackArgs(args) {
  const closeoutArgs = commonCoordinationOptions(args, { includePaths: false });
  pushOption(closeoutArgs, "--owner", args.owner);
  appendTruthyFlag(closeoutArgs, "--all-owned", args.allOwned);
  appendTruthyFlag(closeoutArgs, "--all-lanes", args.allLanes);
  appendTruthyFlag(closeoutArgs, "--allow-other-node", args.allowOtherNode);
  appendFalseyFlag(closeoutArgs, "--no-release", args.releaseOwned);
  appendFalseyFlag(closeoutArgs, "--no-repair-stale", args.repairStale);
  return closeoutArgs;
}

function repairFallbackArgs(args) {
  const repairArgs = [String(args.action ?? "legacy-hash")];
  repairArgs.push(...commonCoordinationOptions(args));
  pushOption(repairArgs, "--owner", args.owner);
  appendTruthyFlag(repairArgs, "--write", args.write);
  appendTruthyFlag(repairArgs, "--dry-run", args.dryRun);
  return repairArgs;
}

function messageFallbackArgs(args) {
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

function ackFallbackArgs(args) {
  return [
    ...commonCoordinationOptions(args, { includePaths: false }),
    ...(args.messageId ? [String(args.messageId)] : []),
    ...(args.id ? [String(args.id)] : []),
  ];
}

function appendTruthyFlag(result, flag, enabled) {
  if (enabled === true) result.push(flag);
}

function appendFalseyFlag(result, flag, value) {
  if (value === false) result.push(flag);
}

export { coordinationCommandFallbackArgs };
