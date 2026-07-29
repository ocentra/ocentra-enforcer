import process from "node:process";

export function parseWorkspaceTestArgs(argv) {
  const packages = [];
  const testArgs = [];
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--package") packages.push(argv[++index]);
    else if (arg?.startsWith("--test-threads=")) testArgs.push(arg);
    else if (arg === "--help" || arg === "-h") {
      console.log("Usage: node scripts/check-cargo-workspace-tests.mjs [--package <name> ...]");
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return {
    packageFilter: packages.length === 0 ? null : packages,
    testArgs,
  };
}
