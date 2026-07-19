export function classifyChangedFiles(changedFiles) {
  const normalized = changedFiles.map((file) => file.replaceAll("\\", "/"));
  const docsOnly = normalized.length > 0 && normalized.every((file) =>
    file === "README.md" || file === "CHANGELOG.md" || file.startsWith("docs/"),
  );
  const graphContractChanged = normalized.some((file) =>
    file === "Cargo.toml"
      || file === "Cargo.lock"
      || file === "rust-toolchain.toml"
      || file.startsWith(".github/")
      || file.startsWith("scripts/ci/"),
  );
  return { normalized, docsOnly, graphContractChanged };
}
