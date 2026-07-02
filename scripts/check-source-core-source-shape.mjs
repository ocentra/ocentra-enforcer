import fs from "node:fs";
import { matchesAnyGlob, normalizeRel } from "../src/path-utils.mjs";
import {
  collectPolicyFiles,
  countLines,
  finding,
} from "./check-source-core-helpers.mjs";
import {
  inspectPythonShape,
  inspectRustShape,
  inspectTypeScriptShape,
} from "./check-source-shape-scanners.mjs";

function collectSourceShapeFindings(root, config, scope = { mode: "all" }) {
  const policies = config.sourceShapePolicies ?? [
    {
      roots: ["src", "apps"],
      extensions: [".ts", ".tsx"],
      kind: "typescript",
      maxClasses: 1,
      maxExports: 35,
      maxFunctionLines: 80,
      maxLines: 1000,
    },
    {
      roots: ["packages"],
      extensions: [".ts", ".tsx"],
      kind: "typescript",
      maxClasses: 1,
      maxExports: 45,
      maxFunctionLines: 80,
      maxLines: 1000,
    },
    {
      roots: ["src", "crates"],
      extensions: [".rs"],
      kind: "rust",
      maxFunctionLines: 80,
      maxFunctions: 18,
      maxLines: 1000,
      maxTypes: 24,
    },
    {
      roots: ["src", "apps", "packages", "tools"],
      extensions: [".py"],
      kind: "python",
      maxClasses: 4,
      maxFunctionLines: 80,
      maxFunctions: 30,
      maxLines: 800,
    },
  ];

  const findings = [];
  for (const policy of policies) {
    for (const file of collectPolicyFiles(root, config, policy, scope)) {
      const rel = normalizeRel(root, file);
      const text = fs.readFileSync(file, "utf8");
      const effectivePolicy = applySourceShapeOverrides(config, rel, policy);
      if (effectivePolicy.kind === "rust") {
        findings.push(...inspectRustShape(root, file, text, effectivePolicy));
      } else if (effectivePolicy.kind === "python") {
        findings.push(...inspectPythonShape(root, file, text, effectivePolicy));
      } else {
        findings.push(
          ...inspectTypeScriptShape(root, file, text, effectivePolicy),
        );
      }
      const lines = countLines(text);
      if (lines > effectivePolicy.maxLines) {
        findings.push(
          finding(
            root,
            file,
            effectivePolicy.maxLines + 1,
            "SRC-1.1",
            `file has ${lines} lines; maximum is ${effectivePolicy.maxLines}`,
            null,
          ),
        );
        findings.push(
          finding(
            root,
            file,
            effectivePolicy.maxLines + 1,
            "SRC-2.1",
            `file has ${lines} lines; maximum is ${effectivePolicy.maxLines}`,
            null,
          ),
        );
      }
    }
  }
  return findings;
}

function applySourceShapeOverrides(config, rel, policy) {
  let effectivePolicy = { ...policy };
  for (const override of config.sourceShapeOverrides ?? []) {
    const matchesPath =
      override.path === rel ||
      (Array.isArray(override.paths) && override.paths.includes(rel));
    const matchesGlob =
      (typeof override.glob === "string" &&
        matchesAnyGlob(rel, [override.glob])) ||
      (Array.isArray(override.globs) && matchesAnyGlob(rel, override.globs));
    if (!matchesPath && !matchesGlob) continue;
    const {
      path: _path,
      paths: _paths,
      glob: _glob,
      globs: _globs,
      note: _note,
      ...limits
    } = override;
    effectivePolicy = { ...effectivePolicy, ...limits };
  }
  return effectivePolicy;
}

export {
  applySourceShapeOverrides,
  collectSourceShapeFindings,
  inspectPythonShape,
  inspectRustShape,
  inspectTypeScriptShape,
};
