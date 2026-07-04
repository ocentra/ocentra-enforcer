import {
  mapLiteralRiskFindings,
  runLiteralRiskScan,
} from "./literal-risk.mjs";

function resolveLiteralRiskScope(scope) {
  if (scope?.mode === "files") {
    return { mode: "files", files: scope.files ?? [] };
  }
  return scope ?? { mode: "all" };
}

export function collectLiteralRiskStandaloneFindings({
  root,
  config,
  args,
  scope,
}) {
  const resolvedScope = resolveLiteralRiskScope(scope);
  const scan = runLiteralRiskScan({
    root,
    files: resolvedScope.mode === "files" ? resolvedScope.files : [],
    config,
    args,
  });
  return {
    scope: resolvedScope,
    findings: mapLiteralRiskFindings(scan, root, {
      hardCategories: args.hardCategories,
      hardRuleIds: args.hardRuleIds,
    }),
  };
}
