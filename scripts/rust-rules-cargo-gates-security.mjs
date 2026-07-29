import { RULES } from "../src/rule-metadata.mjs";
import {
  commandExists,
  configuredCargoCommand,
} from "./rust-rules-cargo-command.mjs";

/** Runs configured Cargo security gates for the workspace. */
export function runCargoSecurityGates(root, config, policies) {
  const violations = [];
  const cargoDenyPolicy = policies[4];
  if (cargoDenyPolicy.enabled) {
    if (!commandExists("cargo-deny")) {
      violations.push({
        ruleId: "RR-11.2",
        severity: cargoDenyPolicy.severity,
        title: RULES["RR-11.2"].title,
        detail: "cargo-deny is required but not installed or not on PATH.",
        file: ".",
        line: 1,
        snippet: RULES["RR-11.2"].snippet,
        source: null,
      });
    } else {
      violations.push(
        ...configuredCargoCommand(
          root,
          config,
          "cargoDeny",
          config.requireCargoDeny,
          "cargo",
          ["deny", "check"],
          "RR-11.1",
        ),
      );
    }
  }

  const cargoAuditPolicy = policies[5];
  if (cargoAuditPolicy.enabled) {
    if (!commandExists("cargo-audit")) {
      violations.push({
        ruleId: "RR-11.3",
        severity: cargoAuditPolicy.severity,
        title: RULES["RR-11.3"].title,
        detail: "cargo-audit is enabled but not installed or not on PATH.",
        file: ".",
        line: 1,
        snippet: RULES["RR-11.3"].snippet,
        source: null,
      });
    } else {
      violations.push(
        ...configuredCargoCommand(
          root,
          config,
          "cargoAudit",
          config.requireCargoAudit,
          "cargo",
          ["audit"],
          "RR-11.3",
        ),
      );
    }
  }
  return violations;
}
