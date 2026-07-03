export function buildLiteralRiskScanCommand(invocation, rootPath, scannerOptions, explicitFiles) {
  const command = [
    ...invocation.args,
    "scan",
    "--root",
    rootPath,
    "--json",
    "--min-score",
    String(scannerOptions.minScore),
  ];
  if (scannerOptions.includeLow) command.push("--include-low");
  if (scannerOptions.includeIgnored) command.push("--include-ignored");
  if (scannerOptions.includeUnknownCode) command.push("--include-unknown-code");
  if (!scannerOptions.respectGitignore) command.push("--no-respect-gitignore");
  if (scannerOptions.failAbove !== null && scannerOptions.failAbove !== undefined) {
    command.push("--fail-above", String(scannerOptions.failAbove));
  }
  if (scannerOptions.maxFileBytes !== null && scannerOptions.maxFileBytes !== undefined) {
    command.push("--max-file-bytes", String(scannerOptions.maxFileBytes));
  }
  if (explicitFiles.length > 0) command.push("--files", ...explicitFiles);
  return command;
}
