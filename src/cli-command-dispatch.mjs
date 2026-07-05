import path from "node:path";
import process from "node:process";
import {
  emitAlwaysOk,
  emitJson,
  emitMaybeOk,
  emitPrintedReport,
  emitText,
} from "./cli-output.mjs";

function createCheckOptions(args, root, config, checkName) {
  return {
    root,
    config,
    json: args.json,
    rawScope: args.scope,
    checkName,
    configPath: args.configPath,
    profile: args.profile,
    checkConfigPath: args.checkConfigPath,
    output: args.output,
    dryRun: args.dryRun,
    staged: args.staged,
    tracked: args.tracked,
    strictEmptyTestTrees: args.strictEmptyTestTrees,
    literalRiskMinScore: args.literalRiskMinScore,
    literalRiskIncludeLow: args.literalRiskIncludeLow,
    literalRiskIncludeIgnored: args.literalRiskIncludeIgnored,
    literalRiskIncludeUnknownCode: args.literalRiskIncludeUnknownCode,
    literalRiskRespectGitignore: args.literalRiskRespectGitignore,
    literalRiskMaxFileBytes: args.literalRiskMaxFileBytes,
    literalRiskFailAbove: args.literalRiskFailAbove,
    literalRiskHardCategories: args.literalRiskHardCategories,
    literalRiskHardRuleIds: args.literalRiskHardRuleIds,
  };
}

async function handleProofCommand(argv, runtime) {
  const report = runtime.runProofCli(argv.slice(3), {
    packRoot: runtime.packRoot,
    defaultRoot: process.cwd(),
  });
  if (report.json) emitJson(report.result);
  else emitText(report.text);
  return report.exitCode;
}

async function handleCoordinationCommand(argv, runtime) {
  await runtime.runCoordinationCli(argv.slice(3));
  return process.exitCode ?? 0;
}

function handleArchitectureCommand(argv, runtime) {
  const report = runtime.runArchitectureCli(argv.slice(3), runtime.cliDeps);
  return emitPrintedReport({
    json: report.json,
    report: report.result,
    printer: runtime.printCheckReport,
  });
}

const RAW_COMMAND_HANDLERS = {
  proof: handleProofCommand,
  coordination: handleCoordinationCommand,
  ledger: handleCoordinationCommand,
  architecture: handleArchitectureCommand,
};

function handleInitCommand({ args, runtime }) {
  const report = runtime.createInitReport(args);
  if (!report.dryRun) runtime.applyInitReport(report);
  return emitAlwaysOk({
    json: args.json,
    report,
    printer: runtime.printInitReport,
  });
}

function handleCodexInstallCommand({ args, runtime }) {
  const report = runtime.applyCodexInstallReport(
    runtime.createCodexInstallReport(args),
  );
  return emitAlwaysOk({
    json: args.json,
    report,
    printer: runtime.printCodexInstallReport,
  });
}

function handleCodexUninstallCommand({ args, runtime }) {
  const report = runtime.applyCodexUninstallReport(
    runtime.createCodexUninstallCliReport(args),
  );
  return emitAlwaysOk({
    json: args.json,
    report,
    printer: runtime.printCodexUninstallReport,
  });
}

function handleCodexDoctorCommand({ args, runtime }) {
  const report = runtime.createCodexDoctorReport(args);
  return emitPrintedReport({
    json: args.json,
    report,
    printer: runtime.printCodexDoctorReport,
  });
}

function handleRouteCommand({ args, root, config, runtime }) {
  const report = runtime.routeRules({
    root,
    configPath: args.configPath,
    profile: args.profile ?? config.profileName,
    scope: args.scope.mode === "all" ? "workspace" : args.scope.mode,
    files: args.scope.files ?? [],
    crateName: args.scope.crateName,
    base: args.scope.base,
    head: args.scope.head,
    ruleId: args.routeRuleId,
  });
  emitJson(report);
  return 0;
}

function handleRunCommand({ args, root, config, runtime }) {
  const report = runtime.runHarness({
    root,
    profile: args.profile,
    tool: args.runTool,
    language: args.languages?.[0],
    harness: config.harness,
    command: args.runCommand,
    runId: args.runId,
    crateName: args.crateName,
    packageName: args.packageName,
    domain: args.domain,
    tags: args.tag ? [args.tag] : undefined,
  });
  return emitPrintedReport({
    json: args.json,
    report,
    printer: runtime.printRunReport,
  });
}

function handleRunsCommand({ args, root, config, runtime }) {
  const report = runtime.runRunsCommand(args, root, config);
  return emitMaybeOk({
    json: args.json,
    report,
    printer: (value) => runtime.printRunsReport(args.runsCommand, value),
  });
}

function handleVerifyCommand({ args, root, config, runtime }) {
  const report = runtime.runEnforcerVerify(
    {
      root,
      config,
      rawScope: args.scope,
      configPath: args.configPath,
      profile: args.profile,
      languages: args.languages,
      verifyMode: args.verifyMode,
    },
    runtime.cliDeps,
  );
  return emitPrintedReport({
    json: args.json,
    report,
    printer: runtime.printCheckReport,
  });
}

function handleAdviseCommand({ args, root, config, runtime }) {
  if (args.adviseTarget !== "literals") {
    throw new Error("ocentra-enforcer advise currently supports only literals");
  }
  return handleCheckCommand({
    args: { ...args, checkName: "literal-risk" },
    root,
    config,
    runtime,
  });
}

function handleCheckCommand({ args, root, config, runtime }) {
  const report = runtime.runEnforcerCheck(
    createCheckOptions(args, root, config, args.checkName),
    runtime.cliDeps,
  );
  return emitPrintedReport({
    json: args.json,
    report,
    printer: runtime.printCheckReport,
  });
}

function handleExplainCommand({ args, runtime }) {
  const report = runtime.explainRule(args.explainRuleId);
  if (args.json) emitJson(report);
  else {
    console.log(`${report.ruleId} ${report.title}`);
    console.log(`Rule: ${report.anchor}`);
    console.log(`Fix: ${report.snippet}`);
  }
  return 0;
}

function handleDoctorCommand({ args, root, config, runtime }) {
  const scope = runtime.resolveScope(root, config, args.scope);
  const report = runtime.doctor(root, config, scope);
  if (args.json) emitJson(report);
  else runtime.printDoctor(report);
  return report.ok ? 0 : 1;
}

function handleScanCommand({ args, root, config, runtime }) {
  const report = runtime.runEnforcerScan(
    {
      root,
      config,
      rawScope: args.scope,
      command: args.command,
      scanOnly: args.scanOnly,
      languages: args.languages,
      profile: args.profile,
    },
    runtime.cliDeps,
  );
  return emitPrintedReport({
    json: args.json,
    report,
    printer: runtime.printScanReport,
  });
}

const COMMAND_HANDLERS = {
  init: handleInitCommand,
  "codex-install": handleCodexInstallCommand,
  "codex-uninstall": handleCodexUninstallCommand,
  "codex-doctor": handleCodexDoctorCommand,
  route: handleRouteCommand,
  run: handleRunCommand,
  runs: handleRunsCommand,
  verify: handleVerifyCommand,
  advise: handleAdviseCommand,
  check: handleCheckCommand,
  explain: handleExplainCommand,
  doctor: handleDoctorCommand,
};

function createCommandContext(args, runtime) {
  const root = path.resolve(args.root);
  const config = runtime.loadConfig(root, args.configPath, args.profile);
  return { args, root, config, runtime };
}

export async function runCliMain(argv = process.argv, runtime) {
  try {
    const rawHandler = RAW_COMMAND_HANDLERS[argv[2]];
    if (rawHandler) return await rawHandler(argv, runtime);

    const args = runtime.parseArgs(argv);
    if (args.help) {
      console.log(runtime.usage());
      return 0;
    }

    const handler = COMMAND_HANDLERS[args.command] ?? handleScanCommand;
    return handler(createCommandContext(args, runtime));
  } catch (error) {
    console.error(
      `Ocentra Enforcer internal error: ${error instanceof Error ? error.message : String(error)}`,
    );
    return 2;
  }
}
