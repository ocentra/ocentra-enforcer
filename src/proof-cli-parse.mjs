function splitList(value) {
  return String(value ?? "")
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

const VALUE_FLAG_HANDLERS = new Map([
  ["--root", (parsed, value) => { parsed.root = value ?? parsed.root; }],
  ["--profile", (parsed, value) => { parsed.profile = value ?? parsed.profile; }],
  ["--proof", (parsed, value) => { parsed.proofId = value ?? null; }],
  ["--proof-id", (parsed, value) => { parsed.proofId = value ?? null; }],
  ["--proofs", (parsed, value) => { parsed.proofIds = splitList(value ?? ""); }],
  ["--plan", (parsed, value) => { parsed.plan = value ?? null; }],
  ["--capability", (parsed, value) => { parsed.capability = value ?? null; }],
  ["--run-id", (parsed, value) => { parsed.runId = value ?? null; }],
  ["--artifact", (parsed, value) => { parsed.artifact = value ?? null; }],
  ["--limit", (parsed, value) => { parsed.limit = Number(value); }],
  ["--limit-bytes", (parsed, value) => { parsed.limitBytes = Number(value); }],
  ["--diagnostic-limit", (parsed, value) => { parsed.diagnosticLimit = Number(value); }],
  ["--legacy-path", (parsed, value) => { parsed.legacyPaths = splitList(value ?? ""); }],
  ["--legacy-paths", (parsed, value) => { parsed.legacyPaths = splitList(value ?? ""); }],
  ["--status", (parsed, value) => { parsed.status = value ?? null; }],
  ["--tag", (parsed, value) => { parsed.tags.push(value ?? ""); }],
  ["--claim-id", (parsed, value) => { parsed.claimId = value ?? null; }],
  ["--script-root", (parsed, value) => { parsed.scriptRoot = value ?? null; }],
]);

const BOOLEAN_FLAG_HANDLERS = new Map([
  ["--include-scripts", (parsed) => { parsed.includeScripts = true; }],
  ["--json", (parsed) => { parsed.json = true; }],
  ["--dry-run", (parsed) => { parsed.dryRun = true; }],
  ["--write", (parsed) => { parsed.write = true; }],
  ["--include-all-scripts", (parsed) => { parsed.includeAllScripts = true; }],
  ["--pin", (parsed) => { parsed.pin = true; }],
  ["--pr-ready", (parsed) => { parsed.prReady = true; }],
  ["--allow-dirty", (parsed) => { parsed.allowDirty = true; }],
]);

export function parseProofCli(tokens, defaultRoot) {
  const cliCommand = tokens[0] && !tokens[0].startsWith("-") ? tokens.shift() : "route";
  const parsed = defaultProofCliState(cliCommand, defaultRoot);
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token === "--") {
      parsed.commandArgs = tokens.slice(index + 1);
      break;
    }
    if (token === "--files") {
      index = consumeFiles(tokens, index, parsed.files);
      continue;
    }
    if (!applyFlag(parsed, token, tokens, index) && token.startsWith("-")) {
      throw new Error(`Unknown proof argument: ${token}`);
    }
    if (VALUE_FLAG_HANDLERS.has(token)) index += 1;
  }
  return finalizeProofCli(parsed);
}

function defaultProofCliState(cliCommand, root) {
  return {
    cliCommand,
    root,
    profile: "strict",
    proofId: null,
    proofIds: [],
    files: [],
    plan: null,
    capability: null,
    commandArgs: [],
    runId: null,
    artifact: null,
    limit: null,
    limitBytes: null,
    diagnosticLimit: null,
    includeScripts: false,
    legacyPaths: [],
    dryRun: false,
    status: null,
    tags: [],
    json: false,
    pin: false,
    prReady: false,
    allowDirty: false,
    claimId: null,
    write: false,
    scriptRoot: null,
    includeAllScripts: false,
  };
}

function consumeFiles(tokens, index, files) {
  for (let fileIndex = index + 1; fileIndex < tokens.length; fileIndex += 1) {
    if (tokens[fileIndex].startsWith("-")) return fileIndex - 1;
    files.push(tokens[fileIndex]);
    index = fileIndex;
  }
  return index;
}

function applyFlag(parsed, token, tokens, index) {
  const value = tokens[index + 1];
  const valueHandler = VALUE_FLAG_HANDLERS.get(token);
  if (valueHandler) {
    valueHandler(parsed, value);
    return true;
  }
  const booleanHandler = BOOLEAN_FLAG_HANDLERS.get(token);
  if (booleanHandler) {
    booleanHandler(parsed);
    return true;
  }
  return false;
}

function finalizeProofCli(parsed) {
  return {
    ...parsed,
    cliCommand: parsed.cliCommand ?? "route",
    scope: parsed.files.length > 0 ? "files" : "workspace",
    command: parsed.commandArgs,
    tags: parsed.tags.filter(Boolean),
  };
}
