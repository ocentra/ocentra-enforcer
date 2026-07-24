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

function targetEntry(packageName, target) {
  const kind = target.kind.find((candidate) => TEST_TARGET_KINDS.has(candidate));
  if (!kind) return null;
  const selectorFlag = TEST_TARGET_KINDS.get(kind);
  const isLibrary = kind === "lib" || kind === "proc-macro";
  return {
    packageName,
    targetName: target.name,
    kind,
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
      if (!matchesTargetArtifact(entryName, names, entry.kind, root === debugRoot)) continue;
      fs.rmSync(path.join(root, entryName), { force: true });
    }
  }
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
  const removableExtension = extension === "" || [".d", ".exe", ".pdb"].includes(extension);
  if (!removableExtension) return false;
  return [...names].some((name) => targetArtifactName(fileName, name));
}

function targetArtifactName(fileName, name) {
  return fileName === `${name}.exe`
    || fileName === name
    || fileName.startsWith(`${name}-`)
    || fileName.startsWith(`${name}.`);
}
