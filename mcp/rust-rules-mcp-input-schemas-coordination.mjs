#!/usr/bin/env node

function coordinationInputSchema(extra = {}) {
  return {
    type: "object",
    additionalProperties: false,
    properties: {
      ...BASE_COORDINATION_PROPERTIES,
      ...extra,
    },
  };
}

function coordinationActionInputSchema(action, description) {
  const actions = Array.isArray(action) ? action : [action];
  return coordinationInputSchema({
    action: {
      type: "string",
      enum: actions,
      description,
    },
  });
}

const BASE_COORDINATION_PROPERTIES = {
  root: {
    type: "string",
    description: "Target repository root for future adapter-aware checks.",
  },
  stateRoot: {
    type: "string",
    description:
      "Exact coordination hub root for repair/import overrides. Defaults to OCENTRA_LEDGER_HOME/<hub>.",
  },
  hub: {
    type: "string",
    description: "Generic hub name used when stateRoot is not explicit.",
  },
  lane: { type: "string", description: "Lane id." },
  paths: {
    type: "array",
    items: { type: "string" },
    description: "Exact changed or claimed paths.",
  },
  changedPaths: {
    type: "array",
    items: { type: "string" },
    description: "Changed paths for guard/health decisions.",
  },
  reason: { type: "string" },
  summary: { type: "string" },
  owner: {
    type: "string",
    description:
      "Optional writer id to preserve for coordination repair stale-claims.",
  },
  operation: {
    type: "string",
    enum: ["inspect", "edit", "commit", "push", "rebase", "merge", "pr_ready"],
    description:
      "Operation-aware guard mode. Defaults to commit for guard/health with paths and edit for claim.",
  },
  lockKind: {
    type: "string",
    enum: ["writeLock", "globalWriteLock", "branchLease", "workReservation"],
    description:
      "Claim kind. Normal exact-file writes use writeLock; singleton paths promote to globalWriteLock.",
  },
  onConflict: {
    type: "string",
    enum: ["fail", "intent"],
    description:
      "When claim is blocked, fail immediately or append an editIntent queue record.",
  },
  claimGroup: {
    type: "string",
    description:
      "Optional group key so source/generated files or related paths lock together.",
  },
  waitMs: {
    type: "number",
    description:
      "Reserved wait budget for future polling; current v1 records intent instead of blocking.",
  },
  from: {
    type: "string",
    description:
      "Optional sender lane for coordination messages. Defaults to the hub identity default lane.",
  },
  to: { type: "string", description: "Message recipient lane/address." },
  subject: {
    type: "string",
    description:
      "Optional message subject. The wire event stores subject as a body prefix for compatibility.",
  },
  body: { type: "string" },
  message: { type: "string", description: "Alias for body." },
  messageId: { type: "string" },
  taskId: { type: "string" },
  state: { type: "string" },
  sessionId: { type: "string" },
  action: {
    type: "string",
    description: "Coordination action for aggregate tools such as peer or mail.",
  },
  peer: { type: "string" },
  peerUrl: { type: "string" },
  url: { type: "string" },
  name: { type: "string" },
  token: { type: "string" },
  tokenEnv: { type: "string" },
  mode: { type: "string", enum: ["pull", "push", "both"] },
  host: { type: "string" },
  port: { type: "number" },
  keepLatest: { type: "number" },
  projectId: { type: "string" },
  repoRoot: { type: "string" },
  worktreeRoot: { type: "string" },
  cwd: { type: "string" },
  gitRemote: { type: "string" },
  branch: { type: "string" },
  commit: { type: "string" },
  codexThreadId: { type: "string" },
  codexSessionId: { type: "string" },
  stateFile: { type: "string" },
  peek: { type: "boolean" },
  dryRun: { type: "boolean" },
  write: { type: "boolean" },
  focused: { type: "boolean" },
  allowPrimaryWithoutClaims: { type: "boolean" },
  allowMergeRisks: { type: "boolean" },
  all: { type: "boolean" },
  allOwned: { type: "boolean" },
  allLanes: { type: "boolean" },
  allowOtherNode: { type: "boolean" },
  releaseOwned: { type: "boolean" },
  repairStale: { type: "boolean" },
  limit: { type: "number" },
};

export { coordinationActionInputSchema, coordinationInputSchema };
