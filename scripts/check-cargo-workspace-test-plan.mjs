import fs from "node:fs";
import path from "node:path";

const TEST_TARGET_KINDS = new Map([
  ["lib", "--lib"],
  ["proc-macro", "--lib"],
  ["bin", "--bin"],
  ["example", "--example"],
  ["test", "--test"],
  ["bench", "--bench"],
]);

const PROCESS_ISOLATED_TARGETS = new Set([
  "enforcer-memory/property_parser_contracts",
  "enforcer-syntax/property_parser_contracts",
]);

const TARGET_ARTIFACT_REMOVE_OPTIONS = Object.freeze({
  force: true,
  recursive: true,
  maxRetries: 6,
  retryDelay: 250,
});

/** Maximum number of non-isolated targets grouped into one package-local batch. */
export const DEFAULT_TEST_TARGET_BATCH_SIZE = 8;

/** Builds a deterministic one-target-at-a-time workspace test plan. */
export function workspaceTestPlan(packages) {
  return packages.flatMap((pkg) => {
    const targets = pkg.targets
      .map((target) => targetEntry(pkg.name, target))
      .filter(Boolean)
      .sort((left, right) => left.selector.localeCompare(right.selector));
    return targets;
  });
}

/** Groups a complete target plan into bounded, package-local Cargo invocations. */
export function workspaceTestBatches(
  plan,
  batchSize = DEFAULT_TEST_TARGET_BATCH_SIZE,
) {
  if (!Number.isSafeInteger(batchSize) || batchSize < 1) {
    throw new Error("Cargo test target batch size must be a positive integer.");
  }
  const batches = [];
  for (const entry of plan) {
    const current = batches.at(-1);
    if (
      !current ||
      entry.processIsolated ||
      current.entries.some((candidate) => candidate.processIsolated) ||
      current.packageName !== entry.packageName ||
      current.entries.length >= batchSize
    ) {
      batches.push({
        packageName: entry.packageName,
        entries: [entry],
      });
      continue;
    }
    current.entries.push(entry);
  }
  return batches.map((batch) => ({
    ...batch,
    selectorArgs: batch.entries.flatMap((entry) => entry.selectorArgs),
    selector: batch.entries.map((entry) => entry.selector).join(" "),
  }));
}

function targetEntry(packageName, target) {
  const kind = target.kind.find((candidate) =>
    TEST_TARGET_KINDS.has(candidate),
  );
  if (!kind) return null;
  const selectorFlag = TEST_TARGET_KINDS.get(kind);
  const isLibrary = kind === "lib" || kind === "proc-macro";
  return {
    packageName,
    targetName: target.name,
    kind,
    processIsolated: PROCESS_ISOLATED_TARGETS.has(
      `${packageName}/${target.name}`,
    ),
    selectorArgs: isLibrary ? [selectorFlag] : [selectorFlag, target.name],
    selector: isLibrary ? selectorFlag : `${selectorFlag} ${target.name}`,
  };
}

/** Removes only generated artifacts for the target that just completed. */
export function cleanupTargetArtifacts(targetDirectory, entry) {
  const names = targetNames(entry.targetName);
  const debugRoot = path.join(targetDirectory, "debug");
  const roots = [
    path.join(debugRoot, "deps"),
    path.join(debugRoot, "examples"),
    debugRoot,
  ];
  for (const root of roots) {
    if (!fs.existsSync(root)) continue;
    for (const entryName of fs.readdirSync(root)) {
      if (
        !matchesTargetArtifact(entryName, names, entry.kind, root === debugRoot)
      )
        continue;
      removeTargetArtifact(path.join(root, entryName));
    }
  }
}

/** Removes one generated artifact with bounded retries for transient Windows executable locks. */
export function removeTargetArtifact(artifactPath, remove = fs.rmSync) {
  remove(artifactPath, TARGET_ARTIFACT_REMOVE_OPTIONS);
}

function targetNames(targetName) {
  return new Set([
    targetName,
    targetName.replaceAll("-", "_"),
    targetName.replaceAll("_", "-"),
  ]);
}

function matchesTargetArtifact(fileName, names, kind, isDebugRoot) {
  if (isDebugRoot && !["bin", "example"].includes(kind)) return false;
  const extension = path.extname(fileName).toLowerCase();
  const removableExtension =
    extension === "" || [".d", ".exe", ".pdb"].includes(extension);
  if (!removableExtension) return false;
  return [...names].some((name) => targetArtifactName(fileName, name));
}

function targetArtifactName(fileName, name) {
  return (
    fileName === `${name}.exe` ||
    fileName === name ||
    fileName.startsWith(`${name}-`) ||
    fileName.startsWith(`${name}.`)
  );
}
