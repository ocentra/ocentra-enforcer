import fs from "node:fs";
import path from "node:path";
import { repoAbsolute } from "./path-utils.mjs";
import { compactProcessOutput, finding, spawnInRoot } from "../scripts/check-source-core-helpers.mjs";

export function runSbomCheck(root, args) {
  const findings = [];
  const outputRoot = repoAbsolute(root, args.output ?? "target/security");
  if (args.dryRun) return [];
  fs.mkdirSync(outputRoot, { recursive: true });

  if (fs.existsSync(path.join(root, "package.json"))) {
    const npmSbom = spawnInRoot(root, "npm", ["sbom", "--sbom-format=cyclonedx"]);
    if (npmSbom.status !== 0) {
      findings.push(
        finding(root, path.join(root, "package.json"), 1, "NPM-1.12", "npm SBOM generation failed", compactProcessOutput(npmSbom)),
        finding(root, path.join(root, "package.json"), 1, "SBOM-1.1", "npm SBOM generation failed", compactProcessOutput(npmSbom)),
      );
    } else {
      fs.writeFileSync(path.join(outputRoot, "npm-sbom.cdx.json"), npmSbom.stdout, "utf8");
    }
  }

  if (fs.existsSync(path.join(root, "Cargo.toml"))) {
    const cargoMetadata = spawnInRoot(root, "cargo", ["metadata", "--format-version=1", "--locked"]);
    if (cargoMetadata.status !== 0) {
      findings.push(
        finding(root, path.join(root, "Cargo.toml"), 1, "SBOM-1.1", "cargo metadata generation failed", compactProcessOutput(cargoMetadata)),
      );
    } else {
      fs.writeFileSync(path.join(outputRoot, "cargo-metadata.json"), cargoMetadata.stdout, "utf8");
    }
  }

  return findings;
}
