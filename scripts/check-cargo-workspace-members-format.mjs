/** Groups workspace packages into bounded Cargo formatting batches. */
export function cargoFmtBatches(workspacePackages, batchSize = 8) {
  if (!Number.isInteger(batchSize) || batchSize < 1) {
    throw new Error("batchSize must be a positive integer");
  }
  const batches = [];
  for (let index = 0; index < workspacePackages.length; index += batchSize) {
    batches.push(workspacePackages.slice(index, index + batchSize).map((pkg) => pkg.name));
  }
  return batches;
}
