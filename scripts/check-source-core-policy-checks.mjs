import { runGenericScan } from "../src/generic-scanners.mjs";
import {
  isSecretScanExemptFixturePath,
  stagedFiles,
} from "./check-source-core-helpers.mjs";

/** Collects secret-policy findings for the requested source scope. */
export function collectSecretPolicyFindings(root, config, scope, args) {
  let scanScope = scope;
  if (args.staged === true) {
    const files = stagedFiles(root);
    if (files.length === 0) return [];
    scanScope = { mode: "files", files };
  }
  const report = runGenericScan({
    root,
    scope: scanScope,
    config,
    languages: ["common"],
  });
  return (report.violations ?? []).filter(
    (entry) =>
      (entry.ruleId === "SEC-1.1" || entry.ruleId === "SEC-1.2") &&
      !isSecretScanExemptFixturePath(String(entry.file ?? "")),
  );
}
