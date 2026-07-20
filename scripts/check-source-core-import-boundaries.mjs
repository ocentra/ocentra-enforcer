import fs from "node:fs";
import { matchesAnyGlob, normalizeRel } from "../src/path-utils.mjs";
import {
  finding,
  importSpecifier,
  isUnderRoots,
  scopeFilesByExtensions,
} from "./check-source-core-helpers.mjs";

function collectImportBoundaryFindings(root, config, scope = { mode: "all" }) {
  const policies = config.importBoundaryPolicies ?? [];
  if (policies.length === 0) return [];
  const files = scopeFilesByExtensions(
    root,
    scope,
    config,
    new Set([".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts"]),
  );
  const findings = [];
  for (const file of files) {
    const rel = normalizeRel(root, file);
    const lines = fs.readFileSync(file, "utf8").split(/\r?\n/u);
    for (const policy of policies) {
      if (!isUnderRoots(rel, policy.roots ?? [])) continue;
      lines.forEach((line, index) => {
        const spec = importSpecifier(line);
        if (!spec) return;
        const forbidden = matchesAnyGlob(spec, policy.forbiddenImports ?? []);
        const allowed = matchesAnyGlob(spec, policy.allowedImports ?? []);
        if (!forbidden || allowed) return;
        findings.push(
          finding(
            root,
            file,
            index + 1,
            "TS-4.1",
            policy.message ?? `import "${spec}" crosses a configured boundary`,
            line,
          ),
        );
      });
    }
  }
  return findings;
}

export { collectImportBoundaryFindings };
