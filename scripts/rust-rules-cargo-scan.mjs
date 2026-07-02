#!/usr/bin/env node
/*
 * Ocentra Enforcer Rust Cargo and workspace scan engine.
 */
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { RULES } from "../src/rule-metadata.mjs";
import { policyForTool } from "../src/policy.mjs";
import {
  normalizeRel,
  toPosix,
  uniqueSorted,
  contextHas,
  addViolation,
  findCargoManifests,
  packageNameFromManifest,
} from "./rust-rules-path-core.mjs";
import { scanRustFile } from "./rust-rules-source-scan.mjs";

function scanWorkspaceFiles(root, config, scope) {
  const violations = [];
  const cargoToml = path.join(root, "Cargo.toml");
  if (!fs.existsSync(cargoToml) || !config.enforceWorkspaceFiles)
    return violations;

  const required = [
    ["rust-toolchain.toml", "RR-1.1"],
    ["Cargo.lock", "RR-1.2"],
    ["clippy.toml", "RR-1.3"],
    ["deny.toml", "RR-1.4"],
  ];
  for (const [fileName, ruleId] of required) {
    if (!fs.existsSync(path.join(root, fileName))) {
      addViolation(
        violations,
        root,
        path.join(root, fileName),
        1,
        ruleId,
        `${fileName} is missing.`,
      );
    }
  }

  const manifestPaths = manifestPathsForScope(root, config, scope);
  for (const manifest of manifestPaths) {
    scanCargoManifest(root, manifest, config, violations);
  }

  return violations;
}

function manifestPathsForScope(root, config, scope) {
  if (scope.mode === "crate" && scope.manifest) return [scope.manifest];
  if (scope.mode === "files" || scope.mode === "diff") {
    return uniqueSorted(
      scope.files
        .map((file) => nearestCargoManifest(root, file))
        .filter(Boolean),
    );
  }
  return findCargoManifests(root, config).filter(
    (manifest) => !normalizeRel(root, manifest).includes("/target/"),
  );
}

function nearestCargoManifest(root, filePath) {
  const rootPath = path.resolve(root);
  let current = path.dirname(path.resolve(filePath));
  while (true) {
    const relative = path.relative(rootPath, current);
    if (relative.startsWith("..") || path.isAbsolute(relative)) return null;
    const manifest = path.join(current, "Cargo.toml");
    if (fs.existsSync(manifest)) return path.resolve(manifest);
    if (current === rootPath) return null;
    const next = path.dirname(current);
    if (next === current) return null;
    current = next;
  }
}

function scanCargoManifest(root, manifest, config, violations) {
  const cargoText = fs.readFileSync(manifest, "utf8");
  const packageBlock = cargoText.match(
    /(?:^|\n)\s*\[package\]([\s\S]*?)(?:\n\s*\[|$)/u,
  );
  const workspacePackageBlock = cargoText.match(
    /(?:^|\n)\s*\[workspace\.package\]([\s\S]*?)(?:\n\s*\[|$)/u,
  );
  if (
    packageBlock &&
    !/(^|\n)\s*rust-version\s*=\s*"[^"]+"/u.test(packageBlock[1]) &&
    !/(^|\n)\s*rust-version\s*=\s*"[^"]+"/u.test(
      workspacePackageBlock?.[1] ?? "",
    )
  ) {
    addViolation(
      violations,
      root,
      manifest,
      1,
      "RR-1.5",
      "Cargo.toml package does not declare rust-version.",
    );
  }

  if (/(?:^|\n)\s*license\s*=\s*"(?:[^"]*\bA?GPL\b[^"]*)"/iu.test(packageBlock?.[1] ?? "")) {
    addViolation(
      violations,
      root,
      manifest,
      1,
      "RR-9.22",
      "GPL/AGPL package license found.",
    );
  }

  const lockPath = path.join(root, "Cargo.lock");
  if (fs.existsSync(lockPath)) {
    const manifestMtime = fs.statSync(manifest).mtimeMs;
    const lockMtime = fs.statSync(lockPath).mtimeMs;
    if (manifestMtime > lockMtime + 1000) {
      addViolation(
        violations,
        root,
        lockPath,
        1,
        "RR-9.25",
        "Cargo.toml is newer than Cargo.lock.",
      );
    }
  }

  const lines = cargoText.split(/\r?\n/u);
  let currentSection = "";
  const dependencyNamesBySection = new Map();
  const dependencyRequirementsByName = new Map();
  const currentPackageName = packageNameFromManifest(manifest);
  const workspacePackageNames = workspacePackageNamesFromManifests(root, config);
  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const sectionMatch = line.match(/^\s*\[([^\]]+)\]\s*$/u);
    if (sectionMatch) currentSection = sectionMatch[1];
    if (/version\s*=\s*"\*"|=\s*"\*"/u.test(line)) {
      addViolation(
        violations,
        root,
        manifest,
        lineNo,
        "RR-9.1",
        "Wildcard dependency versions are forbidden.",
        line,
      );
    }
    const inDependencySection =
      /^(?:dependencies|dev-dependencies|build-dependencies|target\..+\.dependencies)(?:\.|$)/u.test(
        currentSection,
      );
    const inProductionDependencySection =
      /^(?:dependencies|target\..+\.dependencies)(?:\.|$)/u.test(
        currentSection,
      );
    const dependencyName = dependencyNameFromManifestLine(line);
    if (inDependencySection && dependencyName) {
      if (!dependencyNamesBySection.has(currentSection)) dependencyNamesBySection.set(currentSection, new Set());
      dependencyNamesBySection.get(currentSection).add(dependencyName);
      const dependencyRequirement = dependencyRequirementFromManifestLine(line);
      if (inProductionDependencySection && dependencyRequirement) {
        if (!dependencyRequirementsByName.has(dependencyName)) dependencyRequirementsByName.set(dependencyName, new Set());
        dependencyRequirementsByName.get(dependencyName).add(dependencyRequirement);
      }
      if (
        inProductionDependencySection &&
        workspacePackageNames.has(dependencyName) &&
        dependencyName !== currentPackageName &&
        !/\b(?:path|workspace)\s*=/u.test(line)
      ) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.26",
          `${dependencyName} is a workspace member but is not linked by path/workspace dependency syntax.`,
          line,
        );
      }
      if (!contextHas(lines, idx, "DEPENDENCY-JUSTIFICATION:", 4)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.18",
          `${dependencyName} lacks DEPENDENCY-JUSTIFICATION.`,
          line,
        );
      }
      if (/^(?:dependencies|target\..+\.dependencies)(?:\.|$)/u.test(currentSection) && config.testOnlyCratesSet.has(dependencyName)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.27",
          `${dependencyName} is test-only but appears in production dependencies.`,
          line,
        );
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.28",
          `${dependencyName} must be in dev-dependencies only.`,
          line,
        );
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.29",
          `${dependencyName} must not be a production dependency in a runtime crate.`,
          line,
        );
      }
      if (/^(?:dependencies|target\..+\.dependencies)(?:\.|$)/u.test(currentSection) && /\b(?:syn|quote|proc-macro2|darling|proc-macro-error)\b/u.test(dependencyName) && !contextHas(lines, idx, "PROC-MACRO-DEPENDENCY-JUSTIFICATION:", 4)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.20",
          `${dependencyName} is a proc-macro ecosystem dependency in runtime dependencies without approval.`,
          line,
        );
      }
      if (/\b(?:openssl|openssl-sys|libsqlite3-sys|ring|rusqlite|bindgen|cc|cmake|pkg-config)\b/u.test(dependencyName) && !contextHas(lines, idx, "NATIVE-DEPENDENCY-JUSTIFICATION:", 4)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.21",
          `${dependencyName} is native/build-linked and lacks NATIVE-DEPENDENCY-JUSTIFICATION.`,
          line,
        );
      }
      if (/\b(?:tokio|reqwest|sqlx|diesel|aws-sdk|openssl|rusqlite)\b/u.test(dependencyName) && /\{[^}]*version\s*=/u.test(line) && !/\bdefault-features\s*=\s*false\b/u.test(line)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.17",
          `${dependencyName} must set default-features explicitly to false or document the default feature policy.`,
          line,
        );
      }
    }
    if (
      inDependencySection &&
      /=\s*"(?:>=|>|<=|<)[^"]*"|=\s*"\d+"|version\s*=\s*"(?:>=|>|<=|<)[^"]*"|version\s*=\s*"\d+"/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        manifest,
        lineNo,
        "RR-9.16",
        "Loose dependency version range found.",
        line,
      );
    }
    if (!config.allowGitDependencies && /\bgit\s*=/u.test(line)) {
      addViolation(
        violations,
        root,
        manifest,
        lineNo,
        "RR-9.2",
        "Git dependency found.",
        line,
      );
    }
    if (!config.allowPathDependencies && /\bpath\s*=/u.test(line)) {
      addViolation(
        violations,
        root,
        manifest,
        lineNo,
        "RR-9.3",
        "Path dependency found.",
        line,
      );
    }
    if (/^build-dependencies(?:\.|$)/u.test(currentSection)) {
      const isDependencyLine = /^\s*[\w.-]+\s*=/u.test(line);
      if (isDependencyLine && !contextHas(lines, idx, "BUILD-DEPENDENCY-JUSTIFICATION:", 4)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.30",
          "build-dependency lacks BUILD-DEPENDENCY-JUSTIFICATION.",
          line,
        );
      }
    }
  });

  const prodDeps = new Set([
    ...(dependencyNamesBySection.get("dependencies") ?? []),
    ...[...dependencyNamesBySection.entries()]
      .filter(([section]) => /^target\..+\.dependencies/u.test(section))
      .flatMap(([, names]) => [...names]),
  ]);
  const devDeps = dependencyNamesBySection.get("dev-dependencies") ?? new Set();
  for (const name of devDeps) {
    if (prodDeps.has(name)) {
      addViolation(
        violations,
        root,
        manifest,
        1,
        "RR-9.27",
        `${name} appears in both production and dev dependencies.`,
      );
    }
  }
  for (const [dependencyName, requirements] of dependencyRequirementsByName) {
    if (requirements.size > 1) {
      addViolation(
        violations,
        root,
        manifest,
        1,
        "RR-9.19",
        `Direct dependency ${dependencyName} uses multiple requirements: ${[...requirements].join(", ")}.`,
      );
    }
  }

  const denyPath = path.join(root, "deny.toml");
  if (fs.existsSync(denyPath)) {
    const denyText = fs.readFileSync(denyPath, "utf8");
    if (!/\byanked\s*=\s*"deny"/u.test(denyText)) {
      addViolation(violations, root, denyPath, 1, "RR-9.23", 'deny.toml must deny yanked crate versions.');
    }
    if (!/\bunmaintained\s*=\s*"deny"/u.test(denyText)) {
      addViolation(violations, root, denyPath, 1, "RR-9.24", 'deny.toml must deny unmaintained crates when advisory data is available.');
    }
  }

  const buildRs = path.join(path.dirname(manifest), "build.rs");
  if (!config.allowBuildRs && fs.existsSync(buildRs)) {
    addViolation(
      violations,
      root,
      buildRs,
      1,
      "RR-7.5",
      "build.rs is forbidden by default because it can hide non-deterministic build behavior.",
    );
  }
}

function dependencyNameFromManifestLine(line) {
  const match = line.match(/^\s*([A-Za-z0-9_.-]+)\s*=/u);
  return match?.[1] ?? null;
}

function dependencyRequirementFromManifestLine(line) {
  return (
    line.match(/=\s*"([^"]+)"/u)?.[1] ??
    line.match(/\bversion\s*=\s*"([^"]+)"/u)?.[1] ??
    null
  );
}

function workspacePackageNamesFromManifests(root, config) {
  const names = new Set();
  for (const manifest of findCargoManifests(root, config)) {
    const name = packageNameFromManifest(manifest);
    if (name) names.add(name);
  }
  return names;
}

function loadCargoMetadata(root) {
  const result = spawnSync(
    "cargo",
    ["metadata", "--no-deps", "--format-version", "1"],
    {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 32 * 1024 * 1024,
      shell: false,
    },
  );
  if (result.error) throw result.error;
  if ((result.status ?? 1) !== 0) return null;
  return JSON.parse(result.stdout);
}

function scanCargoMetadata(root, config, scope) {
  const violations = [];
  if (!fs.existsSync(path.join(root, "Cargo.toml"))) return violations;
  if (!commandExists("cargo")) return violations;
  const metadata = loadCargoMetadata(root);
  if (!metadata) return violations;
  const workspaceRoot = toPosix(metadata.workspace_root);
  const scopedManifests = new Set(
    manifestPathsForScope(root, config, scope).map((manifest) =>
      toPosix(manifest),
    ),
  );
  const packageFilter =
    scope.mode === "crate"
      ? (packageInfo) => packageInfo.name === scope.crateName
      : scope.mode === "files" || scope.mode === "diff"
        ? (packageInfo) =>
            scopedManifests.has(toPosix(packageInfo.manifest_path))
        : () => true;
  const packages = (metadata.packages ?? []).filter(packageFilter);
  const workspacePackageNames = new Set((metadata.packages ?? []).map((packageInfo) => packageInfo.name));

  for (const packageInfo of packages) {
    const blockedForPackage = new Set(
      config.blockedProtocolDependencies[packageInfo.name] ?? [],
    );
    for (const dependency of packageInfo.dependencies ?? []) {
      if (
        (dependency.source ?? "").startsWith("git+") &&
        !config.allowedGitDependenciesSet.has(dependency.name)
      ) {
        addViolation(
          violations,
          root,
          packageInfo.manifest_path,
          1,
          "RR-9.2",
          "Git dependency found in cargo metadata.",
        );
      }

      const dependencyPath = dependency.path ?? null;
      if (dependencyPath !== null) {
        const normalizedPath = toPosix(dependencyPath);
        if (!normalizedPath.startsWith(workspaceRoot)) {
          addViolation(
            violations,
            root,
            packageInfo.manifest_path,
            1,
            "RR-9.4",
            "Path dependency points outside the workspace root.",
          );
        }
      }

      if (dependencyPath === null && dependency.req.trim() === "*") {
        addViolation(
          violations,
          root,
          packageInfo.manifest_path,
          1,
          "RR-9.1",
          "Wildcard registry dependency version found.",
        );
      }

      if (blockedForPackage.has(dependency.name)) {
        addViolation(
          violations,
          root,
          packageInfo.manifest_path,
          1,
          "RR-9.4",
          `${packageInfo.name} must not depend on ${dependency.name}.`,
        );
      }

      if (
        config.runtimeCratesSet.has(packageInfo.name) &&
        config.testOnlyCratesSet.has(dependency.name) &&
        dependency.kind !== "dev"
      ) {
        addViolation(
          violations,
          root,
          packageInfo.manifest_path,
          1,
          "RR-9.29",
          "Runtime crate depends on test-only crate outside dev-dependencies.",
        );
      }
      if (workspacePackageNames.has(dependency.name) && dependency.path === null && dependency.source !== null) {
        addViolation(
          violations,
          root,
          packageInfo.manifest_path,
          1,
          "RR-9.26",
          `Workspace member ${packageInfo.name} depends on ${dependency.name} by registry version instead of path/workspace linkage.`,
        );
      }
    }
  }

  const directReqs = new Map();
  for (const packageInfo of packages) {
    for (const dependency of packageInfo.dependencies ?? []) {
      if (
        (dependency.path ?? null) !== null ||
        dependency.kind === "dev" ||
        (dependency.source ?? "").startsWith("git+")
      )
        continue;
      if (!directReqs.has(dependency.name))
        directReqs.set(dependency.name, new Set());
      directReqs.get(dependency.name).add(dependency.req);
    }
  }
  for (const [dependencyName, reqs] of directReqs) {
    if (reqs.size > 1) {
      addViolation(
        violations,
        root,
        ".",
        1,
        "RR-9.5",
        `Direct registry dependency ${dependencyName} uses multiple requirements: ${[...reqs].join(", ")}.`,
      );
      addViolation(
        violations,
        root,
        ".",
        1,
        "RR-9.19",
        `Direct registry dependency ${dependencyName} uses duplicate requirements: ${[...reqs].join(", ")}.`,
      );
    }
  }

  return violations;
}

function runScanner(root, config, scope) {
  const violations = [];
  violations.push(...scanWorkspaceFiles(root, config, scope));
  violations.push(...scanCargoMetadata(root, config, scope));
  for (const filePath of scope.files) {
    violations.push(...scanRustFile(root, filePath, config));
    if (config.failFast && violations.length > 0) break;
  }
  return violations;
}

function commandExists(command) {
  const result = spawnSync(command, ["--version"], {
    encoding: "utf8",
    stdio: "pipe",
    shell: false,
  });
  return result.status === 0;
}

function runCommand(root, command, args, ruleId, env = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    env: { ...process.env, ...env },
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    shell: false,
  });
  if (result.status === 0) return [];
  const output = [result.stdout, result.stderr]
    .filter(Boolean)
    .join("\n")
    .trim();
  return [
    {
      ruleId,
      title: RULES[ruleId].title,
      detail: `${command} ${args.join(" ")} failed with exit code ${result.status ?? "unknown"}.`,
      file: ".",
      line: 1,
      snippet: RULES[ruleId].snippet,
      source: output.slice(0, 4000),
    },
  ];
}

function shouldRunCargoForScope(scope, config) {
  if (scope.mode === "all" || scope.mode === "crate") return true;
  if (scope.mode === "files") return config.cargoOnFileScope;
  if (scope.mode === "diff") return config.cargoOnDiffScope;
  return false;
}

function cargoPackageArgs(scope) {
  return scope.mode === "crate"
    ? ["--package", scope.crateName]
    : ["--workspace"];
}

function configuredCargoCommand(
  root,
  config,
  toolId,
  defaultEnabled,
  command,
  args,
  ruleId,
  env = {},
) {
  const toolPolicy = policyForTool(toolId, config, {
    enabled: defaultEnabled,
    severity: "error",
  });
  if (!toolPolicy.enabled) return [];
  return runCommand(root, command, args, ruleId, env).map((finding) => ({
    ...finding,
    severity: toolPolicy.severity,
  }));
}

function strongestEnabledSeverity(policies) {
  const enabled = policies
    .filter((policy) => policy.enabled)
    .map((policy) => policy.severity);
  if (enabled.includes("error")) return "error";
  if (enabled.includes("warning")) return "warning";
  return "info";
}

function runCargoGates(root, config, scope) {
  const violations = [];
  if (!fs.existsSync(path.join(root, "Cargo.toml"))) return violations;
  if (!shouldRunCargoForScope(scope, config)) return violations;

  const cargoToolPolicies = [
    policyForTool("cargoFmt", config, { enabled: config.runCargoFmt, severity: "error" }),
    policyForTool("cargoClippy", config, { enabled: config.runCargoClippy, severity: "error" }),
    policyForTool("cargoTest", config, { enabled: config.runCargoTest, severity: "error" }),
    policyForTool("cargoDoc", config, { enabled: config.runCargoDoc, severity: "error" }),
    policyForTool("cargoDeny", config, { enabled: config.requireCargoDeny, severity: "error" }),
    policyForTool("cargoAudit", config, { enabled: config.requireCargoAudit, severity: "error" }),
  ];
  if (!cargoToolPolicies.some((policy) => policy.enabled)) return violations;

  const packageArgs = cargoPackageArgs(scope);
  const cargoFmtPolicy = cargoToolPolicies[0];
  if (cargoFmtPolicy.enabled) {
    violations.push(
      ...configuredCargoCommand(root, config, "cargoFmt", true, "cargo", ["fmt", ...packageArgs, "--all", "--check"], "RR-10.1"),
    );
  }

  const cargoClippyPolicy = cargoToolPolicies[1];
  if (cargoClippyPolicy.enabled) {
    violations.push(
      ...configuredCargoCommand(
        root,
        config,
        "cargoClippy",
        true,
        "cargo",
        ["clippy", ...packageArgs, "--all-targets", "--all-features", "--", "-D", "warnings"],
        "RR-10.2",
      ),
    );
  }

  const cargoTestPolicy = cargoToolPolicies[2];
  if (cargoTestPolicy.enabled) {
    const testArgs = ["test", ...packageArgs, "--all-features"];
    if (config.cargoTestThreads !== null) {
      testArgs.push("--", `--test-threads=${config.cargoTestThreads}`);
    }
    violations.push(
      ...configuredCargoCommand(
        root,
        config,
        "cargoTest",
        true,
        "cargo",
        testArgs,
        "RR-10.3",
      ),
    );
  }

  const cargoDocPolicy = cargoToolPolicies[3];
  if (cargoDocPolicy.enabled) {
    violations.push(
      ...configuredCargoCommand(
        root,
        config,
        "cargoDoc",
        config.runCargoDoc,
        "cargo",
        ["doc", ...packageArgs, "--all-features", "--no-deps"],
        "RR-10.4",
        {
          RUSTDOCFLAGS:
            "-D warnings -D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls -D missing_docs",
        },
      ),
    );
  }

  const cargoDenyPolicy = cargoToolPolicies[4];
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

  const cargoAuditPolicy = cargoToolPolicies[5];
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


export {
  scanWorkspaceFiles,
  manifestPathsForScope,
  nearestCargoManifest,
  scanCargoManifest,
  dependencyNameFromManifestLine,
  dependencyRequirementFromManifestLine,
  workspacePackageNamesFromManifests,
  loadCargoMetadata,
  scanCargoMetadata,
  runScanner,
  commandExists,
  runCommand,
  shouldRunCargoForScope,
  cargoPackageArgs,
  configuredCargoCommand,
  strongestEnabledSeverity,
  runCargoGates,
};
