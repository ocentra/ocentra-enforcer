function splitList(value) {
  return String(value ?? "")
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

export function stripNullish(value) {
  return Object.fromEntries(
    Object.entries(value).filter(([, entry]) => entry !== null && entry !== undefined),
  );
}

export function parseProofCli(tokens, defaultRoot) {
  const cliCommand = tokens[0] && !tokens[0].startsWith("-") ? tokens.shift() : "route";
  const parsed = {
    cliCommand,
    root: defaultRoot,
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

  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token === "--") {
      parsed.commandArgs = tokens.slice(index + 1);
      break;
    }
    if (token === "--root") parsed.root = tokens[++index] ?? parsed.root;
    else if (token === "--profile") parsed.profile = tokens[++index] ?? parsed.profile;
    else if (token === "--proof" || token === "--proof-id") parsed.proofId = tokens[++index] ?? null;
    else if (token === "--proofs") parsed.proofIds = splitList(tokens[++index] ?? "");
    else if (token === "--plan") parsed.plan = tokens[++index] ?? null;
    else if (token === "--capability") parsed.capability = tokens[++index] ?? null;
    else if (token === "--run-id") parsed.runId = tokens[++index] ?? null;
    else if (token === "--artifact") parsed.artifact = tokens[++index] ?? null;
    else if (token === "--limit") parsed.limit = Number(tokens[++index]);
    else if (token === "--limit-bytes") parsed.limitBytes = Number(tokens[++index]);
    else if (token === "--diagnostic-limit") parsed.diagnosticLimit = Number(tokens[++index]);
    else if (token === "--include-scripts") parsed.includeScripts = true;
    else if (token === "--legacy-path" || token === "--legacy-paths") parsed.legacyPaths = splitList(tokens[++index] ?? "");
    else if (token === "--status") parsed.status = tokens[++index] ?? null;
    else if (token === "--tag") parsed.tags.push(tokens[++index] ?? "");
    else if (token === "--claim-id") parsed.claimId = tokens[++index] ?? null;
    else if (token === "--script-root") parsed.scriptRoot = tokens[++index] ?? null;
    else if (token === "--json") parsed.json = true;
    else if (token === "--dry-run") parsed.dryRun = true;
    else if (token === "--write") parsed.write = true;
    else if (token === "--include-all-scripts") parsed.includeAllScripts = true;
    else if (token === "--pin") parsed.pin = true;
    else if (token === "--pr-ready") parsed.prReady = true;
    else if (token === "--allow-dirty") parsed.allowDirty = true;
    else if (token === "--files") {
      for (let fileIndex = index + 1; fileIndex < tokens.length; fileIndex += 1) {
        if (tokens[fileIndex].startsWith("-")) {
          index = fileIndex - 1;
          break;
        }
        parsed.files.push(tokens[fileIndex]);
        index = fileIndex;
      }
    } else if (token.startsWith("-")) {
      throw new Error(`Unknown proof argument: ${token}`);
    }
  }

  return {
    ...parsed,
    cliCommand: parsed.cliCommand ?? "route",
    scope: parsed.files.length > 0 ? "files" : "workspace",
    command: parsed.commandArgs,
    tags: parsed.tags.filter(Boolean),
  };
}

export function formatProofReport(command, result) {
  if (command === "route") {
    return `Proof route: ${result.proofs.length} proof(s), docs=${result.docs.length}`;
  }
  if (command === "run") {
    return `Proof ${result.proofRun.proofId} ${result.proofRun.status}: ${result.proofRun.runId}`;
  }
  if (command === "import-legacy") {
    return `Imported legacy proof ${result.proofRun.proofId} ${result.proofRun.status}: ${result.proofRun.runId}`;
  }
  if (command === "migrate-legacy") {
    return `Generated ${result.generatedProofCount} legacy proof definition(s) for profile ${result.profile}: dryRun=${result.dryRun}`;
  }
  if (command === "parity") {
    return `Proof parity ${result.coverage}: deletionReady=${result.deletionReady}`;
  }
  if (command === "status") {
    return result.runs
      .map((run) => `${run.runId} ${run.status} ${run.proofId}`)
      .join("\n");
  }
  if (command === "artifact") return result.text ?? result.message ?? "";
  if (command === "diagnostics") {
    return (result.diagnostics ?? [])
      .map((diagnostic) => `${diagnostic.ruleId}: ${diagnostic.message}`)
      .join("\n");
  }
  return JSON.stringify(result, null, 2);
}
