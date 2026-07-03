import { resolveScope } from "../scripts/rust-rules-path-core.mjs";
import {
  mapLiteralRiskFindings,
  runLiteralRiskScan,
} from "./literal-risk.mjs";

export function collectLiteralRiskStandaloneFindings({
  root,
  config,
  args,
  scope,
}) {
  const resolvedScope = resolveScope(root, config, scope ?? { mode: "all" });
  const scan = runLiteralRiskScan({
    root,
    files: resolvedScope.files ?? [],
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
