import { consumeCommand, applyCommandHead } from "./rust-rules-scan-core-args-command.mjs";
import {
  collectFilesFromManifest,
} from "./rust-rules-scan-core-args-files.mjs";
import { collectParsedTokens } from "./rust-rules-scan-core-args-parse.mjs";

export function defaultArgs() {
  return {
    command: "scan",
    root: process.cwd(),
    rootExplicit: false,
    configPath: null,
    scanOnly: false,
    json: false,
    help: false,
    explainRuleId: null,
    profile: null,
    languages: null,
    adapters: null,
    dryRun: false,
    force: false,
    runTool: null,
    runCommand: [],
    runId: null,
    routeRuleId: null,
    checkName: null,
    adviseTarget: null,
    checkConfigPath: null,
    output: null,
    staged: false,
    tracked: false,
    strictEmptyTestTrees: false,
    codexConfigPath: null,
    ledgerRoot: null,
    mcpServerName: "ocentra-enforcer",
    installSkill: true,
    installGlobalAgents: true,
    workspace: false,
    runsCommand: null,
    artifact: null,
    limit: null,
    limitBytes: null,
    severity: null,
    status: null,
    file: null,
    tag: null,
    crateName: null,
    packageName: null,
    domain: null,
    literalRiskMinScore: null,
    literalRiskIncludeLow: null,
    literalRiskIncludeIgnored: null,
    literalRiskIncludeUnknownCode: null,
    literalRiskRespectGitignore: null,
    literalRiskMaxFileBytes: null,
    literalRiskFailAbove: null,
    literalRiskHardCategories: null,
    literalRiskHardRuleIds: null,
    verifyMode: "local",
    scope: { mode: "all" },
  };
}

export function parseArgs(argv) {
  const args = defaultArgs();
  const tokens = argv.slice(2);
  args.command = consumeCommand(tokens);
  applyCommandHead(args, tokens);

  const { explicitFiles, explicitFileManifests } = collectParsedTokens(tokens, args);

  for (const manifestPath of explicitFileManifests) {
    explicitFiles.push(...collectFilesFromManifest(manifestPath, args.root));
  }

  if (explicitFiles.length > 0) {
    args.scope = { mode: "files", files: explicitFiles };
  } else if (args.crateName) {
    args.scope = { mode: "crate", crateName: args.crateName };
  } else if (args.scope.mode === "diff" && args.scope.base && args.scope.head) {
    args.scope = { mode: "diff", base: args.scope.base, head: args.scope.head };
  } else if (args.workspace) {
    args.scope = { mode: "all" };
  }
  if (args.scope.mode === "diff" && (!args.scope.base || !args.scope.head)) {
    throw new Error("Diff scope requires --base <sha> --head <sha>.");
  }
  return args;
}
