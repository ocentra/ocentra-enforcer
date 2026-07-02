import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { lineNumberAt, matchesAnyGlob, normalizeRel, repoAbsolute, uniqueSorted } from "./path-utils.mjs";
import { CHECK_RULES } from "./check-metadata.mjs";
import {
  collectDependencyPolicyFindings,
  collectPackageDeterminismFindings,
  runSbomCheck,
} from "./check-governance-packages.mjs";
export { collectDependencyPolicyFindings, collectPackageDeterminismFindings, runSbomCheck };
import {
  collectSourceFiles,
  diffFiles,
  escapeRegExp,
  finding,
} from "../scripts/check-source-core-helpers.mjs";

export function collectCiIntegrityFindings(root) {
  const findings = [];
  findings.push(...collectCiSubprocessCaptureFindings(root));
  findings.push(...collectNoImplicitShellFindings(root));

  const workflowRoot = path.join(root, ".github", "workflows");
  const packageText = fs.existsSync(path.join(root, "package.json"))
    ? fs.readFileSync(path.join(root, "package.json"), "utf8")
    : "";
  const localCiText = fs.existsSync(path.join(root, "scripts", "ci-local.mjs"))
    ? fs.readFileSync(path.join(root, "scripts", "ci-local.mjs"), "utf8")
    : "";
  const ciSurfaceText = `${packageText}\n${localCiText}`;
  if (!fs.existsSync(workflowRoot)) {
    return findings;
  }
  for (const file of collectSourceFiles(workflowRoot, [".yml", ".yaml"])) {
    const text = fs.readFileSync(file, "utf8");
    const lines = text.split(/\r?\n/u);
    if (!/\bpull_request\s*:/u.test(text) || !/\bpush\s*:/u.test(text) || !/\bbranches\s*:\s*\[[^\]]*\bmain\b/u.test(text)) {
      findings.push(
        finding(
          root,
          file,
          1,
          "CI-1.15",
          "workflow must run on pull_request and pushes to main",
          null,
        ),
      );
    }
    if (!/^permissions\s*:/mu.test(text)) {
      findings.push(
        finding(root, file, 1, "CI-1.14", "workflow is missing explicit permissions block", null),
      );
    }
    const hasUbuntu = /\bubuntu-latest\b/u.test(text);
    const hasWindows = /\bwindows-latest\b/u.test(text);
    const hasMacos = /\bmacos-latest\b/u.test(text);
    if (!(hasUbuntu && hasWindows && hasMacos)) {
      findings.push(
        finding(
          root,
          file,
          1,
          "CI-1.16",
          "workflow matrix must include ubuntu-latest, windows-latest, and macos-latest",
          null,
        ),
      );
    }
    lines.forEach((line, index) => {
      if (/continue-on-error\s*:\s*true/u.test(line)) {
        findings.push(
          finding(root, file, index + 1, "CI-1.11", "continue-on-error bypass found", line),
        );
      }
      if (/\|\|\s*true\b/u.test(line)) {
        findings.push(
          finding(root, file, index + 1, "CI-1.12", "shell exit-code bypass found", line),
        );
      }
      if (/\brun\s*:\s*npm\s+install\b/u.test(line)) {
        findings.push(
          finding(root, file, index + 1, "CI-1.1", "workflow uses npm install instead of npm ci", line),
        );
        findings.push(
          finding(root, file, index + 1, "NPM-1.2", "workflow uses npm install instead of npm ci", line),
        );
      }
      const action = line.match(/^\s*-\s+uses\s*:\s*([^\s#]+)\s*$/u);
      if (action && !isPinnedActionReference(action[1])) {
        findings.push(
          finding(root, file, index + 1, "CI-1.13", `workflow action is not pinned by full commit SHA: ${action[1]}`, line),
        );
      }
      if (/\brust-rules\b/u.test(line) && !/compatibility alias/iu.test(line)) {
        findings.push(
          finding(root, file, index + 1, "CI-1.18", "workflow calls legacy rust-rules command directly", line),
        );
      }
    });
    if (/\bpackage-lock\.json\b|package\.json\b|npm\b/u.test(text) && !/\brun\s*:\s*npm\s+ci\b/u.test(text)) {
      findings.push(
        finding(root, file, 1, "CI-1.1", "workflow does not run npm ci", null),
      );
      findings.push(
        finding(root, file, 1, "NPM-1.2", "workflow does not run npm ci", null),
      );
    }
    const usesLocalParity = /\brun\s*:\s*npm\s+run\s+ci:local\b/u.test(text);
    if (!usesLocalParity) {
      findings.push(
        finding(root, file, 1, "CI-1.17", "workflow does not run npm run ci:local parity gate", null),
      );
    }
    const ciText = usesLocalParity ? `${text}\n${ciSurfaceText}` : text;
    for (const requirement of CI_COMMAND_REQUIREMENTS) {
      if (!requirement.pattern.test(ciText)) {
        findings.push(
          finding(root, file, 1, requirement.ruleId, requirement.detail, null),
        );
      }
    }
  }
  const branchProtectionPath = path.join(root, "docs", "BRANCH_PROTECTION.md");
  if (!fs.existsSync(branchProtectionPath)) {
    findings.push(
      finding(root, root, 1, "CI-1.19", "docs/BRANCH_PROTECTION.md is required", null),
    );
    findings.push(
      finding(root, root, 1, "CI-1.20", "required checks policy must include Enforcer", null),
    );
  } else {
    const branchProtection = fs.readFileSync(branchProtectionPath, "utf8");
    if (!/\bbranch protection\b|\brequired checks\b/iu.test(branchProtection)) {
      findings.push(
        finding(root, branchProtectionPath, 1, "CI-1.19", "branch protection document must describe required checks", null),
      );
    }
    if (!/\bocentra enforcer\b|\bci:local\b|\benforcer\b/iu.test(branchProtection)) {
      findings.push(
        finding(root, branchProtectionPath, 1, "CI-1.20", "required checks document must include Enforcer", null),
      );
    }
  }
  return findings;
}

export function collectRepoGovernanceFindings(root) {
  const findings = [];
  const codeownersPath = findCodeownersPath(root);
  if (!codeownersPath) {
    findings.push(
      finding(root, root, 1, "REPO-1.1", "CODEOWNERS file is missing", null),
    );
  } else {
    const codeowners = fs.readFileSync(codeownersPath, "utf8");
    for (const [ruleId, requiredPatterns] of [
      ["REPO-1.2", ["rules/**"]],
      ["REPO-1.3", ["scripts/**", "src/**", "mcp/**"]],
      ["REPO-1.4", ["schemas/**", "profiles/**", "adapters/**"]],
      ["REPO-1.5", [".github/workflows/**"]],
    ]) {
      const missing = requiredPatterns.filter(
        (pattern) => !codeownersIncludesPattern(codeowners, pattern),
      );
      if (missing.length > 0) {
        findings.push(
          finding(
            root,
            codeownersPath,
            1,
            ruleId,
            `CODEOWNERS missing protection for ${missing.join(", ")}`,
            null,
          ),
        );
      }
    }
  }

  const packageFindings = collectPackageDeterminismFindings(root);
  for (const packageFinding of packageFindings) {
    const ruleId =
      packageFinding.ruleId === "NPM-1.1"
        ? "REPO-1.6"
        : packageFinding.ruleId === "NPM-1.4"
          ? "REPO-1.7"
          : packageFinding.ruleId === "NPM-1.5"
            ? "REPO-1.8"
            : packageFinding.ruleId === "NPM-1.3"
              ? "REPO-1.9"
              : packageFinding.ruleId;
    findings.push({
      ...packageFinding,
      ruleId,
      title: CHECK_RULES[ruleId]?.title ?? packageFinding.title,
      snippet: CHECK_RULES[ruleId]?.snippet ?? packageFinding.snippet,
    });
  }

  for (const requiredDoc of REPO_GOVERNANCE_DOCS) {
    const docPath = path.join(root, requiredDoc.path);
    if (!fs.existsSync(docPath)) {
      findings.push(
        finding(root, root, 1, requiredDoc.ruleId, `${requiredDoc.path} is required`, null),
      );
      continue;
    }
    const text = fs.readFileSync(docPath, "utf8");
    if (requiredDoc.pattern && !requiredDoc.pattern.test(text)) {
      findings.push(
        finding(
          root,
          docPath,
          1,
          requiredDoc.ruleId,
          `${requiredDoc.path} is missing required governance content`,
          null,
        ),
      );
    }
  }

  return findings;
}

export function collectMutationRiskFindings(root, scope = { mode: "all" }) {
  const changedFiles = changedFilesForMutationRisk(root, scope);
  const criticalFiles = changedFiles.filter((file) =>
    matchesAnyGlob(normalizeRel(root, file), POLICY_CRITICAL_PATTERNS),
  );
  return criticalFiles.map((file) =>
    finding(
      root,
      file,
      1,
      "ENF-2.1",
      `policy-critical file changed: ${normalizeRel(root, file)}`,
      `Required proof set: ${MUTATION_RISK_REQUIRED_PROOFS.join("; ")}`,
    ),
  );
}

const CI_SUBPROCESS_CAPTURE_ROOTS = ["tests", "scripts", "src", "mcp"];
const CI_SUBPROCESS_CAPTURE_EXTENSIONS = [".js", ".mjs", ".cjs", ".ts", ".tsx"];

function collectNoImplicitShellFindings(root) {
  const findings = [];
  for (const relRoot of ["scripts", "src", "mcp"]) {
    const absRoot = path.join(root, relRoot);
    if (!fs.existsSync(absRoot)) continue;
    for (const file of collectSourceFiles(absRoot, CI_SUBPROCESS_CAPTURE_EXTENSIONS)) {
      const rel = normalizeRel(root, file);
      const lines = fs.readFileSync(file, "utf8").split(/\r?\n/u);
      lines.forEach((line, index) => {
        if (!/\bshell\s*:\s*(?:true|process\.platform\s*===\s*["']win32["'])/u.test(line)) {
          return;
        }
        findings.push(
          finding(
            root,
            file,
            index + 1,
            "HAR-2.15",
            `${rel} enables shell execution for a harness command`,
            line,
          ),
        );
      });
    }
  }
  return findings;
}

function collectCiSubprocessCaptureFindings(root) {
  const findings = [];
  for (const relRoot of CI_SUBPROCESS_CAPTURE_ROOTS) {
    const absRoot = path.join(root, relRoot);
    if (!fs.existsSync(absRoot)) continue;
    for (const file of collectSourceFiles(absRoot, CI_SUBPROCESS_CAPTURE_EXTENSIONS)) {
      const rel = normalizeRel(root, file);
      if (/^tests\/fixtures\/enforcer\//u.test(rel)) continue;
      const text = fs.readFileSync(file, "utf8");
      if (!/\bspawnSync\s*\(/u.test(text)) continue;
      if (!parsesSubprocessPipeJson(text)) continue;
      if (hasCiSafeSubprocessCapture(text)) continue;
      const index = text.search(/\bspawnSync\s*\(/u);
      findings.push(
        finding(
          root,
          file,
          lineNumberAt(text, index < 0 ? 0 : index),
          "CI-1.21",
          `${rel} parses child-process stdout/stderr as JSON without file-backed capture or an explicit large maxBuffer`,
          null,
        ),
      );
    }
  }
  return findings;
}

function parsesSubprocessPipeJson(text) {
  return (
    /\bJSON\.parse\s*\([^)]*\b(?:stdout|stderr)\b/u.test(text) ||
    /\bJSON\.parse\s*\([^)]*\.\s*(?:stdout|stderr)\b/u.test(text)
  );
}

function hasCiSafeSubprocessCapture(text) {
  return (
    /\bspawnCli\s*\(/u.test(text) ||
    /\bmaxBuffer\s*:\s*(?:[1-9]\d{6,}|\d+\s*\*\s*1024\s*\*\s*1024|[A-Z][A-Z0-9_]*_MAX_BUFFER)\b/u.test(text) ||
    /\bstdio\s*:\s*\[\s*["']ignore["']\s*,\s*fs\.openSync\s*\(/u.test(text) ||
    /\bfs\.openSync\s*\([^)]*["']w["']\s*\)[\s\S]{0,2400}\bstdio\s*:\s*\[\s*["']ignore["']\s*,\s*\w+Fd\s*,\s*\w+Fd\s*\]/u.test(text)
  );
}

const CI_COMMAND_REQUIREMENTS = [
  {
    ruleId: "CI-1.2",
    pattern: /\bnpm(?:Step)?\(\s*\[\s*["']test["']\s*\]\s*\)|\bnpm\s+test\b|\bnpm\s+run\s+test\b/u,
    detail: "CI parity gate does not run npm test",
  },
  {
    ruleId: "CI-1.3",
    pattern: /\btest:policy\b|\brule-coverage\b|\benforcer:coverage\b/u,
    detail: "CI parity gate does not run rule/policy tests",
  },
  {
    ruleId: "CI-1.4",
    pattern: /\btest:multilang\b|\brust:rules:scan\b|\benforcer:self\b/u,
    detail: "CI parity gate does not run multi-language/self scan coverage",
  },
  {
    ruleId: "CI-1.5",
    pattern: /\btest:mcp\b|\bmcp:smoke\b/u,
    detail: "CI parity gate does not run MCP tests/smoke checks",
  },
  {
    ruleId: "CI-1.6",
    pattern: /\benforcer:self\b|\brust:rules:scan\b/u,
    detail: "CI parity gate does not run Enforcer self-scan",
  },
  {
    ruleId: "CI-1.7",
    pattern: /\benforcer:verify(?::(?:local|ci|fast))?\b|\bverify\s+(?:local|ci|fast)\b|\bpolicy-integrity\b|\benforcer:policy\b/u,
    detail: "CI parity gate does not run schema/policy validation",
  },
  {
    ruleId: "CI-1.8",
    pattern: /\bsecrets\b|\bsecret scan\b|\bscan-staged-secrets\b|\bverify\s+ci\b|\benforcer:verify:ci\b/u,
    detail: "CI parity gate does not run a secret scan",
  },
  {
    ruleId: "CI-1.9",
    pattern: /\bdependency-policy\b|\bverify\s+ci\b|\benforcer:verify:ci\b/u,
    detail: "CI parity gate does not run dependency policy",
  },
  {
    ruleId: "CI-1.10",
    pattern: /\bsbom\b|\bverify\s+ci\b|\benforcer:verify:ci\b/u,
    detail: "CI parity gate does not run SBOM check",
  },
];

const REPO_GOVERNANCE_DOCS = [
  {
    ruleId: "REPO-1.10",
    path: "LICENSE",
    pattern: /\b(?:MIT|Apache|BSD|ISC|MPL|GPL|Proprietary|Copyright)\b/u,
  },
  {
    ruleId: "REPO-1.11",
    path: "SECURITY.md",
    pattern: /\b(?:vulnerability|security|report)\b/iu,
  },
  {
    ruleId: "REPO-1.12",
    path: "CONTRIBUTING.md",
    pattern: /\b(?:rule|validator|fixture|registry|schema)\b/iu,
  },
  {
    ruleId: "REPO-1.13",
    path: "CHANGELOG.md",
    pattern: /\b(?:rule|validator|enforcer|change)\b/iu,
  },
  {
    ruleId: "REPO-1.14",
    path: path.join("docs", "RELEASE_POLICY.md"),
    pattern: /\b(?:tag|sign|publish|release)\b/iu,
  },
];

const POLICY_CRITICAL_PATTERNS = [
  "rules/**",
  "schemas/**",
  "profiles/**",
  "scripts/**",
  "src/policy*",
  "src/checks*",
  "src/generic-scanners*",
  "src/source-policy-scanners*",
  "mcp/**",
  ".github/workflows/**",
  "package.json",
  "package-lock.json",
  "Cargo.toml",
  "Cargo.lock",
  "deny.toml",
  "rust-toolchain.toml",
];

const MUTATION_RISK_REQUIRED_PROOFS = [
  "ocentra-enforcer scan --workspace",
  "ocentra-enforcer check rule-coverage --root <repo>",
  "ocentra-enforcer check policy-integrity --root <repo>",
  "ocentra-enforcer check ci-integrity --root <repo>",
  "ocentra-enforcer check repo-governance --root <repo>",
  "npm test",
  "npm run test:mcp",
];

function changedFilesForMutationRisk(root, scope = { mode: "all" }) {
  if (scope.mode === "files") {
    return uniqueSorted((scope.files ?? []).map((file) => repoAbsolute(root, file)));
  }
  if (scope.mode === "diff") {
    return diffFiles(root, scope.base, scope.head);
  }
  return gitStatusChangedFiles(root);
}

function gitStatusChangedFiles(root) {
  const output = spawnSync("git", ["status", "--porcelain"], {
    cwd: root,
    encoding: "utf8",
    shell: false,
  });
  if (output.status !== 0) return [];
  return uniqueSorted(
    String(output.stdout ?? "")
      .split(/\r?\n/u)
      .map((line) => line.trimEnd())
      .filter(Boolean)
      .map((line) => {
        const rawPath = line.slice(3).trim();
        const renamedPath = rawPath.includes(" -> ")
          ? rawPath.split(" -> ").at(-1)
          : rawPath;
        return repoAbsolute(root, renamedPath.replace(/^"|"$/gu, ""));
      }),
  );
}

function findCodeownersPath(root) {
  for (const rel of [
    "CODEOWNERS",
    ".github/CODEOWNERS",
    "docs/CODEOWNERS",
  ]) {
    const candidate = path.join(root, rel);
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

function codeownersIncludesPattern(text, pattern) {
  return text
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line !== "" && !line.startsWith("#"))
    .some((line) => line.split(/\s+/u)[0] === pattern);
}

function dependencySections(manifest) {
  return [
    ["dependencies", manifest.dependencies],
    ["devDependencies", manifest.devDependencies],
    ["optionalDependencies", manifest.optionalDependencies],
    ["peerDependencies", manifest.peerDependencies],
  ].filter(([, value]) => value && typeof value === "object");
}

function isDeterministicDependencyVersion(value) {
  const version = String(value ?? "").trim();
  return (
    /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/u.test(version) ||
    /^npm:[@A-Za-z0-9._/-]+@\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/u.test(version)
  );
}

function isPinnedActionReference(actionRef) {
  const ref = String(actionRef ?? "").split("@").at(1);
  if (!ref) return false;
  return /^[a-f0-9]{40}$/iu.test(ref);
}

function isSuspiciousDependencyName(name) {
  const normalized = String(name ?? "").toLowerCase();
  return /(?:ocentra|openai|effect|typescript|eslint|vitest|playwright|duckdb)[_-](?:js|lib|safe|new|next)$/u.test(normalized);
}

function isBoundedNodeEngine(value) {
  const engine = String(value ?? "").trim();
  return (
    /^>=\d+(?:\.\d+)?(?:\.\d+)?\s+<\d+(?:\.\d+)?(?:\.\d+)?$/u.test(engine) ||
    /^\d+\.\d+\.\d+$/u.test(engine)
  );
}

function lineForJsonKey(filePath, key) {
  if (!fs.existsSync(filePath)) return 1;
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/u);
  const pattern = new RegExp(`"${escapeRegExp(key)}"\\s*:`, "u");
  const index = lines.findIndex((line) => pattern.test(line));
  return index === -1 ? 1 : index + 1;
}
