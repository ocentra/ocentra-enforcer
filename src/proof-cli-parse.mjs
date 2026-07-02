function splitList(value) {
  return String(value ?? "")
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

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
    if (!applyScalarFlag(parsed, token, tokens, index) && token.startsWith("-")) {
      throw new Error(`Unknown proof argument: ${token}`);
    }
    if (isValueFlag(token)) index += 1;
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

function applyScalarFlag(parsed, token, tokens, index) {
  const value = tokens[index + 1];
  if (token === "--root") parsed.root = value ?? parsed.root;
  else if (token === "--profile") parsed.profile = value ?? parsed.profile;
  else if (token === "--proof" || token === "--proof-id") parsed.proofId = value ?? null;
  else if (token === "--proofs") parsed.proofIds = splitList(value ?? "");
  else if (token === "--plan") parsed.plan = value ?? null;
  else if (token === "--capability") parsed.capability = value ?? null;
  else if (token === "--run-id") parsed.runId = value ?? null;
  else if (token === "--artifact") parsed.artifact = value ?? null;
  else if (token === "--limit") parsed.limit = Number(value);
  else if (token === "--limit-bytes") parsed.limitBytes = Number(value);
  else if (token === "--diagnostic-limit") parsed.diagnosticLimit = Number(value);
  else if (token === "--legacy-path" || token === "--legacy-paths") parsed.legacyPaths = splitList(value ?? "");
  else if (token === "--status") parsed.status = value ?? null;
  else if (token === "--tag") parsed.tags.push(value ?? "");
  else if (token === "--claim-id") parsed.claimId = value ?? null;
  else if (token === "--script-root") parsed.scriptRoot = value ?? null;
  else if (token === "--include-scripts") parsed.includeScripts = true;
  else if (token === "--json") parsed.json = true;
  else if (token === "--dry-run") parsed.dryRun = true;
  else if (token === "--write") parsed.write = true;
  else if (token === "--include-all-scripts") parsed.includeAllScripts = true;
  else if (token === "--pin") parsed.pin = true;
  else if (token === "--pr-ready") parsed.prReady = true;
  else if (token === "--allow-dirty") parsed.allowDirty = true;
  else return false;
  return true;
}

function isValueFlag(token) {
  return [
    "--root",
    "--profile",
    "--proof",
    "--proof-id",
    "--proofs",
    "--plan",
    "--capability",
    "--run-id",
    "--artifact",
    "--limit",
    "--limit-bytes",
    "--diagnostic-limit",
    "--legacy-path",
    "--legacy-paths",
    "--status",
    "--tag",
    "--claim-id",
    "--script-root",
  ].includes(token);
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
