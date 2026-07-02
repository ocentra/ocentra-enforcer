import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

import { normalizeRel, repoAbsolute, toPosix, uniqueSorted } from "./path-utils.mjs";

const SCRIPT_REFERENCE_PATTERNS = [
  "test-results",
  "output/",
  "docs/proof",
  "docs/expectations",
  "docs/plans",
  "packages/schema-domain",
  "packages/agent-protocol-domain",
  "crates/agent-protocol",
  "crates/agent-service",
  "apps/portal",
];

const DEFAULT_LEGACY_PROOF_ROOTS = ["test-results", "output", "docs/proof"];
const LEGACY_ARTIFACT_EXTENSIONS = new Set([
  ".json",
  ".md",
  ".txt",
  ".log",
  ".xml",
  ".sarif",
  ".ndjson",
  ".png",
  ".jpg",
  ".jpeg",
  ".webm",
  ".zip",
]);

function normalizeProofSlug(value) {
  return (
    String(value ?? "proof")
      .toLowerCase()
      .replace(/[^a-z0-9]+/gu, "-")
      .replace(/^-+|-+$/gu, "") || "proof"
  );
}

function projectProfileName(root) {
  return normalizeProofSlug(path.basename(root));
}

function readProfileProofRegistry(packRoot, profileName) {
  const registryPath = path.join(packRoot, "profiles", profileName, "proofs.json");
  if (!fs.existsSync(registryPath)) return null;
  return JSON.parse(fs.readFileSync(registryPath, "utf8"));
}

function mergeProofDefinitions(baseProofs, profileProofs) {
  const merged = new Map();
  for (const proof of baseProofs) merged.set(proof.id, proof);
  for (const proof of profileProofs) merged.set(proof.id, proof);
  return [...merged.values()];
}

function expandProofCommand(command, context) {
  return command.map((part) =>
    String(part)
      .replaceAll("$node", process.execPath)
      .replaceAll("$root", context.root)
      .replaceAll("$packRoot", context.packRoot)
      .replaceAll("$profile", context.profile)
      .replaceAll("$proofId", context.proofId)
      .replaceAll("$runId", context.runId ?? ""),
  );
}

function legacyScriptProofDefinition({ profile, script }) {
  const proofId = `${profile}.${normalizeProofSlug(script.name.replace(/\.mjs$/u, ""))}`;
  return {
    id: proofId,
    title: `Legacy migrated proof: ${script.name}`,
    family: script.family,
    severity: "error",
    appliesTo: ["profile:legacy-project", `plan:${script.planBucket}`, "kind:proof-script"],
    triggers: uniqueSorted([
      "kind:proof-script",
      ...script.proofTypes,
      ...script.capabilities.map((capability) => `capability:${capability}`),
    ]),
    languages: ["common"],
    capabilities: script.capabilities,
    collector: "command",
    docs: [
      "proof/INDEX.md#legacy-script-migration",
      "docs/PROOF_SYSTEM_DESIGN.md#deterministic-migration-sequence",
    ],
    commands: [
      [
        "$node",
        "$packRoot/scripts/profile-proof-runner.mjs",
        "--root",
        "$root",
        "--profile",
        "$profile",
        "--script",
        script.path,
      ],
    ],
    requiredArtifacts: ["proof-run.json", "summary.md", "diagnostics.ndjson"],
    requiredPaths: [],
    claimsProved: [
      `Migrated legacy proof script ${script.path} executed through Enforcer profile ${profile}.`,
      ...legacyScriptClaimSummaries(script, "proved"),
    ],
    claimsNotProved: legacyScriptClaimSummaries(script, "not-proved"),
    ciSupport: script.capabilities.includes("ci"),
    deviceSupport:
      script.capabilities.includes("android-device") ||
      script.capabilities.includes("ios-device") ||
      script.capabilities.includes("manual-required"),
  };
}

function legacyScriptClaimSummaries(script, kind) {
  const summaries = [];
  if (kind === "proved") {
    if (script.claimSemantics.hasExpectationRows) {
      summaries.push("Referenced expectation matrix rows are part of the proof scope.");
    }
    if (script.signals.spawn) {
      summaries.push("Legacy command execution remains captured under bounded Enforcer artifacts.");
    }
    if (script.signals.writesProof) {
      summaries.push("Legacy artifact outputs are captured as proof evidence.");
    }
  } else {
    if (script.capabilities.includes("manual-required")) {
      summaries.push("Manual or physical device behavior is not silently claimed without collected evidence.");
    }
    if (script.claimSemantics.dependsOnPriorProof) {
      summaries.push("Prior proof dependencies must be present and current before this proof can close a plan.");
    }
  }
  return uniqueSorted(summaries);
}

function proofFamilyKeys(args = {}) {
  const families = new Set();
  if (args.plan) families.add(`plan:${args.plan}`);
  if (args.capability) families.add(`capability:${args.capability}`);
  if (args.scope === "workspace") families.add("scope:workspace");
  for (const file of args.files ?? []) {
    for (const family of proofFamilyKeysForFile(file)) families.add(family);
  }
  return families;
}

function proofFamilyKeysForFile(file) {
  const rel = toPosix(file);
  const lower = rel.toLowerCase();
  const families = [];
  if (lower.endsWith(".rs") || lower.includes("cargo.toml")) families.push("language:rust");
  if (/\.[cm]?[jt]sx?$/u.test(lower) || lower.endsWith("package.json")) {
    families.push("language:typescript");
  }
  if (lower.endsWith(".py") || lower.endsWith("pyproject.toml")) {
    families.push("language:python");
  }
  if (/(?:^|\/)(?:test|tests|__tests__)\/|(?:\.test|\.spec)\./u.test(lower)) {
    families.push("kind:test");
  }
  if (lower.startsWith("scripts/test/") || lower.includes("proof")) {
    families.push("kind:proof-script");
  }
  if (lower.includes("android")) families.push("capability:android-device");
  if (lower.includes("ios") || lower.includes("xcode")) {
    families.push("capability:ios-simulator");
  }
  return families;
}

function proofMatchesRoute(proof, args, familyKeys) {
  if (args.capability && !(proof.capabilities ?? []).includes(args.capability)) return false;
  if (
    args.plan &&
    !stringListMatches([...(proof.appliesTo ?? []), ...(proof.triggers ?? [])], args.plan)
  ) {
    return false;
  }
  if (familyKeys.size === 0) return false;
  const proofKeys = new Set([
    `family:${proof.family}`,
    ...(proof.languages ?? []).map((language) => `language:${language}`),
    ...(proof.capabilities ?? []).map((capability) => `capability:${capability}`),
    ...(proof.triggers ?? []),
    ...(proof.appliesTo ?? []),
  ]);
  if (familyKeys.has("scope:workspace")) {
    return proof.appliesTo?.includes("workspace") || proof.family === "claim-integrity";
  }
  for (const key of familyKeys) {
    if (proofKeys.has(key)) return true;
  }
  return false;
}

function compactProofDefinition(proof) {
  return {
    id: proof.id,
    title: proof.title,
    family: proof.family,
    severity: proof.severity,
    collector: proof.collector,
    capabilities: proof.capabilities,
    docs: proof.docs,
  };
}

function describeProofRouteScope(args) {
  if (args.proofId) return { mode: "proof", proofId: args.proofId };
  if (args.files?.length) return { mode: "files", files: args.files };
  if (args.plan) return { mode: "plan", plan: args.plan };
  if (args.capability) return { mode: "capability", capability: args.capability };
  return { mode: args.scope ?? "workspace" };
}

function collectScriptFiles(root) {
  const files = [];
  const stack = [root];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) stack.push(fullPath);
      else if (entry.isFile() && entry.name.endsWith(".mjs")) files.push(fullPath);
    }
  }
  return files.sort((left, right) => left.localeCompare(right));
}

function classifyProofScript(root, filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const rel = normalizeRel(root, filePath);
  const name = path.basename(filePath).toLowerCase();
  const capabilities = scriptCapabilities(name, text);
  const references = SCRIPT_REFERENCE_PATTERNS.filter((pattern) => text.includes(pattern));
  const family = scriptFamily(name, text);
  const proofTypes = scriptProofTypes(name, text);
  const migration = scriptMigrationPlan({
    name,
    text,
    family,
    proofTypes,
    capabilities,
  });
  return {
    path: rel,
    name,
    family,
    planBucket: scriptPlanBucket(name, text),
    proofTypes,
    claimSemantics: scriptClaimSemantics(text),
    migration,
    capabilities,
    signals: {
      spawn: /\bspawn(?:Sync)?\b|\bexec(?:File|Sync)?\b/u.test(text),
      writesProof: /\b(?:writeFile|writeFileSync|appendFile|appendFileSync|writeJson)\s*\(/u.test(text),
      readsProof: readsPriorProofText(text),
      manualOrDevice: /manual-required|physical|ANDROID_SERIAL|\badb\b|ios|simulator|device/iu.test(text),
      importsBuiltOrSchemaParse: /dist\/|await import|Schema\.parse|\.parse\(/u.test(text),
      claimsProved: /claimsProved/u.test(text),
      claimsNotProved: /claimsNotProved|mustNotClaim|notProved/u.test(text),
      rowsCovered: /rowsCovered|matrixRows|expectations/u.test(text),
      writesMarkdown: /\.md['"]|summary\.md|snapshot\.md/u.test(text),
      screenshots: /screenshot|png|jpg|image/iu.test(text),
      adb: /\badb\b|ANDROID_SERIAL/u.test(text),
      playwright: /playwright|\.spec\./u.test(text),
      cargo: /cargo /u.test(text),
      npm: /npm /u.test(text),
      vitest: /vitest/u.test(text),
    },
    references,
    outputRoots: uniqueSorted(
      [...text.matchAll(/(?:test-results|output\/|docs\/proof)[^'"`\s,)]+/gu)].map(
        (match) => match[0],
      ),
    ),
  };
}

function scriptFamily(name, text) {
  const lower = `${name}\n${text}`.toLowerCase();
  if (/android|ios|device|physical|simulator|xcode|adb/u.test(lower)) return "device-manual";
  if (/junit|pytest|vitest|jest|playwright|test-results/u.test(lower)) return "test-report";
  if (/sarif|codeql|security|secret|audit/u.test(lower)) return "security-report";
  if (/parity|contract|boundary|schema/u.test(lower)) return "contract-parity";
  if (/event|network|lan|message|codec/u.test(lower)) return "event-network";
  if (/logging|tracking|custody/u.test(lower)) return "logging-custody";
  if (/release|package|install|billing|payment|production/u.test(lower)) return "release-package";
  return "command";
}

function scriptCapabilities(name, text) {
  const lower = `${name}\n${text}`.toLowerCase();
  const capabilities = ["local"];
  if (lower.includes("ci")) capabilities.push("ci");
  if (lower.includes("windows")) capabilities.push("windows");
  if (lower.includes("linux")) capabilities.push("linux");
  if (lower.includes("macos")) capabilities.push("macos");
  if (lower.includes("wsl")) capabilities.push("wsl");
  if (lower.includes("android")) {
    capabilities.push(lower.includes("emulator") ? "android-emulator" : "android-device");
  }
  if (lower.includes("ios")) {
    capabilities.push(lower.includes("device") ? "ios-device" : "ios-simulator");
  }
  if (lower.includes("browser") || lower.includes("playwright")) capabilities.push("browser");
  if (lower.includes("network") || lower.includes("lan")) capabilities.push("network");
  if (lower.includes("cloud")) capabilities.push("cloud");
  if (lower.includes("manual-required") || lower.includes("physical")) {
    capabilities.push("manual-required");
  }
  return uniqueSorted(capabilities);
}

function scriptPlanBucket(name, text) {
  const lowerName = name.toLowerCase();
  const lowerText = text.toLowerCase();
  const knownBuckets = [
    ["app-install", "app-install-purchase"],
    ["app-game", "app-game"],
    ["browser-game", "browser-game"],
    ["browser", "browser"],
    ["screen-ai", "screen-ai"],
    ["screen-vlm", "screen"],
    ["screen-capture", "screen"],
    ["screen-live", "screen"],
    ["screen-", "screen"],
    ["v0-8", "v0-8-platform-adapters"],
    ["v0-9", "v0-9-household-lan"],
    ["child-android", "child-android"],
    ["child-ios", "child-ios"],
    ["child-macos", "child-macos"],
    ["eventing-network", "eventing-network"],
    ["eventing", "eventing"],
    ["parent-desktop", "parent-desktop"],
    ["production-release", "production-release"],
    ["production-support", "production-support"],
    ["network-remote", "network-remote-delivery"],
    ["network-live", "network-live-capture"],
    ["network-evidence", "network-evidence"],
    ["network-windows", "network-windows"],
    ["tracking", "tracking"],
    ["social", "social"],
    ["local-ai", "local-ai"],
    ["provider-secret", "provider-secret"],
    ["billing-entitlement", "billing-entitlement"],
    ["lan-ai", "lan-ai"],
    ["managed-browser", "managed-browser"],
  ];
  for (const [needle, bucket] of knownBuckets) {
    if (lowerName.includes(needle)) return bucket;
  }
  for (const [needle, bucket] of knownBuckets) {
    if (
      lowerText.includes(`docs/plans/${needle}`) ||
      lowerText.includes(`docs/expectations/${needle}`)
    ) {
      return bucket;
    }
  }
  return name.replace(/\.mjs$/u, "").split("-").slice(0, 2).join("-") || "unclassified";
}

function scriptProofTypes(name, text) {
  const lowerName = name.toLowerCase();
  const lowerText = text.toLowerCase();
  const types = [];
  if (/android|ios|xcode|device|simulator/u.test(lowerName) || /ANDROID_SERIAL|\badb\b/u.test(text)) {
    types.push("device-execution");
  }
  if (/manual-required|physical/u.test(lowerText) || /screenshot|snapshot\.md/iu.test(text)) {
    types.push("manual-evidence");
  }
  if (/claimsproved|claimsnotproved|mustnotclaim|notproved/u.test(lowerText)) {
    types.push("claim-integrity");
  }
  if (/rowscovered|matrixrows|docs\/expectations|pre-ai-proof-matrix/u.test(lowerText)) {
    types.push("expectation-matrix");
  }
  if (readsPriorProofText(text)) types.push("proof-composition");
  if (
    /parity|contract|schema|codec|boundary/u.test(lowerName) ||
    /Schema\.parse|decode[A-Z]\w+\(/u.test(text)
  ) {
    types.push("contract-parity");
  }
  if (/event|network|lan|message|websocket|bridge/u.test(lowerName)) {
    types.push("runtime-event-contract");
  }
  if (/release|package|install|billing|payment|production/u.test(lowerName)) {
    types.push("release-readiness");
  }
  if (
    /sarif|codeql|security|secret|audit/u.test(lowerName) ||
    /sarif|codeql|pip-audit|cargo audit/u.test(lowerText)
  ) {
    types.push("security-report");
  }
  if (
    /junit|pytest|vitest|jest|playwright|cargo-test|npm-test|\.spec\.|\.test\./u.test(
      lowerName,
    ) ||
    /vitest|playwright|cargo test|npm run test/u.test(lowerText)
  ) {
    types.push("test-report");
  }
  if (/\bspawn(?:Sync)?\b|\bexec(?:File|Sync)?\b/u.test(text)) {
    types.push("command-execution");
  }
  if (
    /\b(?:writeFile|writeFileSync|appendFile|appendFileSync|writeJson)\s*\(/u.test(text) ||
    /output\/|docs\/proof|summary\.md/u.test(lowerText)
  ) {
    types.push("artifact-snapshot");
  }
  return uniqueSorted(types.length > 0 ? types : ["command-execution"]);
}

function scriptClaimSemantics(text) {
  const claimKinds = [];
  if (/claimsProved/u.test(text)) claimKinds.push("explicit-claims-proved");
  if (/claimsNotProved|mustNotClaim|notProved/u.test(text)) {
    claimKinds.push("explicit-non-claims");
  }
  if (/rowsCovered|matrixRows|docs\/expectations/u.test(text)) {
    claimKinds.push("expectation-row-coverage");
  }
  if (readsPriorProofText(text)) claimKinds.push("prior-proof-dependency");
  if (/\b(?:writeFile|writeFileSync|appendFile|appendFileSync|writeJson)\s*\(/u.test(text)) {
    claimKinds.push("artifact-producing");
  }
  if (/manual-required|physical|ANDROID_SERIAL|\badb\b|ios|simulator|device/iu.test(text)) {
    claimKinds.push("capability-gated");
  }
  return {
    claimKinds: uniqueSorted(claimKinds),
    hasExplicitClaimList: /claimsProved|claimsNotProved/u.test(text),
    hasNonClaimBoundary: /claimsNotProved|mustNotClaim|notProved/u.test(text),
    hasExpectationRows: /rowsCovered|matrixRows|docs\/expectations/u.test(text),
    dependsOnPriorProof: readsPriorProofText(text),
  };
}

function readsPriorProofText(text) {
  return /readJson|readFile(?:Sync)?\s*\([^)]*(?:proof|test-results)|loadProof|proofManifest/u.test(
    text,
  );
}

function scriptMigrationPlan({ name, text, family, proofTypes, capabilities }) {
  const typeSet = new Set(proofTypes);
  const capabilitySet = new Set(capabilities);
  const lower = `${name}\n${text}`.toLowerCase();
  if (typeSet.has("device-execution") || capabilitySet.has("manual-required")) {
    return {
      template: "DeviceProof<Capability, DeviceSelector, ArtifactPlan>",
      mode: "capability-gated",
      deletionGate:
        "new proof records device identity, unavailable/waived state, required artifacts, and explicit non-claims",
    };
  }
  if (typeSet.has("proof-composition") && typeSet.has("claim-integrity")) {
    return {
      template: "ProofBundleClaim<RequiredProofIds, ClosureMatrix>",
      mode: "compose-existing-proofs",
      deletionGate:
        "new proof rejects stale/missing prior runs and deleted artifact paths",
    };
  }
  if (typeSet.has("contract-parity")) {
    return {
      template: "ContractParityProof<Authority, Mirror, DriftTests>",
      mode: "replace-with-contract-definition",
      deletionGate:
        "new proof checks authority source, generated mirrors, drift tests, and copied-value non-claims",
    };
  }
  if (typeSet.has("runtime-event-contract")) {
    return {
      template: "RuntimeEventProof<Producer, Transport, Consumer>",
      mode: "replace-with-runtime-contract-definition",
      deletionGate:
        "new proof captures command output plus event/transport assertions and authentic-runtime claims",
    };
  }
  if (
    typeSet.has("release-readiness") ||
    family === "release-package" ||
    /production/u.test(lower)
  ) {
    return {
      template: "ReleaseReadinessProof<Workflow, Artifact, ManualGate>",
      mode: "replace-with-release-definition",
      deletionGate:
        "new proof separates CI-reproducible artifacts from manual/platform gates",
    };
  }
  if (typeSet.has("security-report")) {
    return {
      template: "SecurityReportProof<SarifOrAudit, Policy>",
      mode: "replace-with-parser-definition",
      deletionGate:
        "new proof parses SARIF/audit output and ties findings to commit",
    };
  }
  if (typeSet.has("test-report")) {
    return {
      template: "StructuredTestReportProof<Runner, ReportParser>",
      mode: "replace-with-parser-definition",
      deletionGate:
        "new proof parses nonzero tests, failures, skipped/focused tests, and report artifacts",
    };
  }
  return {
    template: "CommandProof<CommandPlan, AssertionPlan>",
    mode: "replace-with-command-definition",
    deletionGate:
      "new proof captures command, bounded logs, structured diagnostics, artifacts, and explicit claims",
  };
}

function buildMigrationMatrix(scripts, limit = 20) {
  const rows = new Map();
  for (const script of scripts) {
    const key = `${script.planBucket}\u0000${script.migration.template}`;
    const current = rows.get(key) ?? {
      planBucket: script.planBucket,
      scriptCount: 0,
      families: {},
      proofTypes: {},
      capabilities: {},
      migrationTemplate: script.migration.template,
      migrationMode: script.migration.mode,
      deletionGate: script.migration.deletionGate,
      representativeScripts: [],
    };
    current.scriptCount += 1;
    current.families[script.family] = (current.families[script.family] ?? 0) + 1;
    for (const proofType of script.proofTypes) {
      current.proofTypes[proofType] = (current.proofTypes[proofType] ?? 0) + 1;
    }
    for (const capability of script.capabilities) {
      current.capabilities[capability] = (current.capabilities[capability] ?? 0) + 1;
    }
    if (current.representativeScripts.length < 5) {
      current.representativeScripts.push(script.path);
    }
    rows.set(key, current);
  }
  return [...rows.values()]
    .sort(
      (left, right) =>
        right.scriptCount - left.scriptCount ||
        left.planBucket.localeCompare(right.planBucket),
    )
    .slice(0, limit);
}

function collectLegacyProofArtifacts(root, args = {}) {
  const explicitPaths = args.legacyPaths?.length ? args.legacyPaths : [];
  const sourceFiles = uniqueSorted(
    (explicitPaths.length > 0
      ? explicitPaths
      : DEFAULT_LEGACY_PROOF_ROOTS.filter((entry) => fs.existsSync(repoAbsolute(root, entry))))
      .flatMap((entry) => collectLegacyArtifactFiles(root, entry)),
  ).slice(0, Number.isFinite(args.limit) ? Math.max(0, args.limit) : 500);
  const artifacts = sourceFiles.map((file) => analyzeLegacyArtifact(root, file));
  const claimsProved = uniqueSorted(artifacts.flatMap((artifact) => artifact.claimsProved));
  const claimsNotProved = uniqueSorted(
    artifacts.flatMap((artifact) => artifact.claimsNotProved),
  );
  const failedArtifacts = artifacts
    .filter((artifact) =>
      ["failed", "manual-required", "unavailable"].includes(artifact.status),
    )
    .map((artifact) => ({
      path: artifact.path,
      status: artifact.status,
      reason: artifact.statusReason,
    }));
  return {
    schemaVersion: 1,
    collectedAt: new Date().toISOString(),
    sourceRoots: explicitPaths.length > 0 ? explicitPaths : DEFAULT_LEGACY_PROOF_ROOTS,
    sourcePaths: artifacts.map((artifact) => artifact.path),
    artifactCount: artifacts.length,
    artifacts,
    failedArtifacts,
    claimsProved,
    claimsNotProved,
    fingerprint: legacyFingerprint(artifacts),
  };
}

function collectLegacyArtifactFiles(root, entry) {
  const absolute = repoAbsolute(root, entry);
  if (!fs.existsSync(absolute)) return [];
  const stat = fs.statSync(absolute);
  if (stat.isFile()) {
    return isLegacyProofArtifactFile(absolute) ? [normalizeRel(root, absolute)] : [];
  }
  if (!stat.isDirectory()) return [];
  const files = [];
  const stack = [absolute];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const child of fs.readdirSync(current, { withFileTypes: true })) {
      const childPath = path.join(current, child.name);
      if (child.isDirectory()) {
        if ([".git", ".enforce", "node_modules", "target", "dist", "build"].includes(child.name)) {
          continue;
        }
        stack.push(childPath);
      } else if (child.isFile() && isLegacyProofArtifactFile(childPath)) {
        files.push(normalizeRel(root, childPath));
      }
    }
  }
  return files;
}

function isLegacyProofArtifactFile(filePath) {
  return LEGACY_ARTIFACT_EXTENSIONS.has(path.extname(filePath).toLowerCase());
}

function analyzeLegacyArtifact(root, relPath) {
  const absolute = repoAbsolute(root, relPath);
  const content = fs.readFileSync(absolute);
  const ext = path.extname(absolute).toLowerCase();
  const text = [".json", ".md", ".txt", ".log", ".xml", ".sarif", ".ndjson"].includes(ext)
    ? content.toString("utf8")
    : "";
  const json = ext === ".json" || ext === ".sarif" ? parseJsonMaybe(text) : null;
  const statusInfo = inferLegacyStatus(json, text);
  return {
    path: normalizeRel(root, absolute),
    kind: ext.replace(/^\./u, "") || "artifact",
    sha256: crypto.createHash("sha256").update(content).digest("hex"),
    byteLength: content.byteLength,
    status: statusInfo.status,
    statusReason: statusInfo.reason,
    claimsProved: uniqueSorted([
      ...claimsFromJson(json, "proved"),
      ...claimsFromMarkdown(text, "proved"),
    ]),
    claimsNotProved: uniqueSorted([
      ...claimsFromJson(json, "not-proved"),
      ...claimsFromMarkdown(text, "not-proved"),
    ]),
  };
}

function inferLegacyStatus(json, text) {
  const value = json && typeof json === "object" ? json : {};
  const status = String(value.status ?? value.result ?? "").toLowerCase();
  if (
    value.ok === false ||
    value.passed === false ||
    value.success === false ||
    ["failed", "fail", "error"].includes(status)
  ) {
    return { status: "failed", reason: "legacy artifact reports a failed status" };
  }
  if (
    ["manual-required", "manual_required"].includes(status) ||
    /manual-required/iu.test(text)
  ) {
    return {
      status: "manual-required",
      reason: "legacy artifact reports manual-required evidence",
    };
  }
  if (["unavailable", "skipped"].includes(status)) {
    return { status: "unavailable", reason: `legacy artifact reports ${status}` };
  }
  if (
    value.ok === true ||
    value.passed === true ||
    value.success === true ||
    ["passed", "pass", "ok"].includes(status)
  ) {
    return { status: "passed", reason: "legacy artifact reports a passing status" };
  }
  return {
    status: "present",
    reason: "legacy artifact exists but does not expose a standard status field",
  };
}

function parseJsonMaybe(text) {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function claimsFromJson(json, kind) {
  if (!json || typeof json !== "object") return [];
  const keys =
    kind === "proved"
      ? ["claimsProved", "provedClaims", "claims", "rowsCovered"]
      : [
          "claimsNotProved",
          "claimsNotProven",
          "notProved",
          "mustNotClaim",
          "nonClaims",
        ];
  return keys.flatMap((key) => normalizeClaimList(json[key]));
}

function normalizeClaimList(value) {
  if (Array.isArray(value)) return value.flatMap((entry) => normalizeClaimList(entry));
  if (value && typeof value === "object") {
    if (typeof value.claim === "string") return [value.claim];
    if (typeof value.id === "string") return [value.id];
    if (typeof value.name === "string") return [value.name];
    return [];
  }
  return typeof value === "string" && value.trim().length > 0 ? [value.trim()] : [];
}

function claimsFromMarkdown(text, kind) {
  if (!text) return [];
  const headerPattern =
    kind === "proved"
      ? /claims?\s+proved|proved\s+claims|what\s+this\s+proves/iu
      : /claims?\s+not\s+proved|non-claims?|what\s+this\s+does\s+not\s+prove|not\s+proved/iu;
  const lines = text.split(/\r?\n/u);
  const claims = [];
  let capturing = false;
  for (const line of lines) {
    if (/^\s{0,3}#{1,6}\s+/u.test(line)) {
      capturing = headerPattern.test(line);
      continue;
    }
    if (!capturing) continue;
    const bullet = /^\s*(?:[-*]|\d+\.)\s+(.+?)\s*$/u.exec(line);
    if (bullet) claims.push(bullet[1]);
    else if (line.trim().length === 0) continue;
    else if (/^\s{0,3}#{1,6}\s+/u.test(line)) capturing = false;
  }
  return claims;
}

function legacyFingerprint(artifacts) {
  const hash = crypto.createHash("sha256");
  for (const artifact of artifacts) {
    hash.update(`${artifact.path}\0${artifact.sha256}\0${artifact.byteLength}\n`);
  }
  return `sha256:${hash.digest("hex")}`;
}

function legacyDiagnostics({ runId, proofId, legacy }) {
  if (legacy.artifacts.length === 0) {
    return [
      {
        runId,
        proofId,
        severity: "error",
        ruleId: "PROOF-LEGACY-MISSING",
        message: "No legacy proof artifacts were found for the selected roots.",
        file: ".",
        line: 1,
      },
    ];
  }
  return legacy.failedArtifacts.map((artifact) => ({
    runId,
    proofId,
    severity: "error",
    ruleId: "PROOF-LEGACY-STATUS",
    message: `${artifact.path} is not a passing legacy proof artifact: ${artifact.reason}`,
    file: artifact.path,
    line: 1,
  }));
}

function sanitizeRelativeArtifactName(relPath) {
  return toPosix(relPath)
    .replace(/^[A-Za-z]:/u, "")
    .replace(/^\/+/u, "")
    .replace(/[^A-Za-z0-9._/-]+/gu, "-");
}

function stringListMatches(values, needle) {
  return values.some(
    (value) => value === needle || value.endsWith(`:${needle}`) || value.includes(needle),
  );
}

function countBy(rows, key) {
  const result = {};
  for (const row of rows) {
    result[row[key] ?? "unknown"] = (result[row[key] ?? "unknown"] ?? 0) + 1;
  }
  return result;
}

function countByNested(rows, accessor) {
  const result = {};
  for (const row of rows) {
    const key = accessor(row) ?? "unknown";
    result[key] = (result[key] ?? 0) + 1;
  }
  return result;
}

function countArrayValues(rows, key) {
  const result = {};
  for (const row of rows) {
    for (const value of row[key] ?? []) {
      result[value] = (result[value] ?? 0) + 1;
    }
  }
  return result;
}

function countCapabilities(scripts) {
  const result = {};
  for (const script of scripts) {
    for (const capability of script.capabilities) {
      result[capability] = (result[capability] ?? 0) + 1;
    }
  }
  return result;
}

function countReferences(scripts) {
  const result = {};
  for (const script of scripts) {
    for (const reference of script.references) {
      result[reference] = (result[reference] ?? 0) + 1;
    }
  }
  return result;
}

function countClaimSignals(scripts) {
  return {
    explicitClaimsProved: scripts.filter((script) => script.claimSemantics.hasExplicitClaimList).length,
    explicitNonClaimBoundary: scripts.filter((script) => script.claimSemantics.hasNonClaimBoundary).length,
    expectationRows: scripts.filter((script) => script.claimSemantics.hasExpectationRows).length,
    priorProofDependencies: scripts.filter((script) => script.claimSemantics.dependsOnPriorProof).length,
    artifactProducing: scripts.filter((script) => script.claimSemantics.claimKinds.includes("artifact-producing")).length,
    capabilityGated: scripts.filter((script) => script.claimSemantics.claimKinds.includes("capability-gated")).length,
  };
}

export {
  projectProfileName,
  readProfileProofRegistry,
  mergeProofDefinitions,
  expandProofCommand,
  legacyScriptProofDefinition,
  proofFamilyKeys,
  proofMatchesRoute,
  compactProofDefinition,
  describeProofRouteScope,
  collectScriptFiles,
  classifyProofScript,
  buildMigrationMatrix,
  collectLegacyProofArtifacts,
  legacyDiagnostics,
  sanitizeRelativeArtifactName,
  countBy,
  countByNested,
  countArrayValues,
  countCapabilities,
  countReferences,
  countClaimSignals,
};
