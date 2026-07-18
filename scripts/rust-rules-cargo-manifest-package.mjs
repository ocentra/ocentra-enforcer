import { addViolation } from "./rust-rules-path-core.mjs";

/** Scans a Cargo package section for package-level policy violations. */
export function scanCargoManifestPackage(root, manifest, cargoText, violations) {
  const packageBlock = cargoText.match(/(?:^|\n)\s*\[package\]([\s\S]*?)(?:\n\s*\[|$)/u);
  const workspacePackageBlock = cargoText.match(/(?:^|\n)\s*\[workspace\.package\]([\s\S]*?)(?:\n\s*\[|$)/u);
  addRustVersionFinding(root, manifest, packageBlock, workspacePackageBlock, violations);
  addLicenseFinding(root, manifest, packageBlock, violations);
}

function addRustVersionFinding(root, manifest, packageBlock, workspacePackageBlock, violations) {
  const packageText = packageBlock?.[1] ?? "";
  const workspaceText = workspacePackageBlock?.[1] ?? "";
  if (packageBlock && !hasRustVersion(packageText) && !hasWorkspaceRustVersion(packageText, workspaceText)) {
    addViolation(violations, root, manifest, 1, "RR-1.5", "Cargo.toml package does not declare rust-version.");
  }
}

function hasRustVersion(packageText) {
  return /(^|\n)\s*rust-version\s*=\s*"[^"]+"/u.test(packageText);
}

function hasWorkspaceRustVersion(packageText, workspaceText) {
  return /(^|\n)\s*rust-version\.workspace\s*=\s*true\b/u.test(packageText) || hasRustVersion(workspaceText);
}

function addLicenseFinding(root, manifest, packageBlock, violations) {
  if (/(?:^|\n)\s*license\s*=\s*"(?:[^"]*\bA?GPL\b[^"]*)"/iu.test(packageBlock?.[1] ?? "")) {
    addViolation(violations, root, manifest, 1, "RR-9.22", "GPL/AGPL package license found.");
  }
}
