import { createLiteralMatchPattern, valueFromSpec } from "./check-source-core-contract-values.mjs";

function loadContract(root, rawContract) {
  const ownerPath = rawContract.ownerPath;
  const values = (rawContract.values ?? []).map((valueSpec) => {
    const text = valueFromSpec(root, ownerPath, valueSpec);
    if (typeof text !== "string" || text.length === 0) {
      throw new Error(
        `${ownerPath}: ${valueSpec.name} must be a non-empty string`,
      );
    }
    return {
      name: valueSpec.name,
      text,
      pattern: createLiteralMatchPattern(text),
    };
  });
  const valueByName = new Map(values.map((value) => [value.name, value.text]));
  const mirrorPaths = [];
  for (const mirror of rawContract.mirrors ?? []) {
    mirrorPaths.push(mirror.path);
    for (const mirrorValueSpec of mirror.values ?? []) {
      const ownerText = valueByName.get(mirrorValueSpec.name);
      if (ownerText === undefined) {
        throw new Error(
          `${mirror.path}: ${mirrorValueSpec.name} does not match an owner value name`,
        );
      }
      const mirrorText = valueFromSpec(root, mirror.path, mirrorValueSpec);
      if (mirrorText !== ownerText) {
        throw new Error(
          `${mirror.path}: ${rawContract.name}.${mirrorValueSpec.name} ${mirrorText} does not match ${ownerPath} ${ownerText}`,
        );
      }
    }
  }
  return {
    ...rawContract,
    allowedPaths: new Set(
      [ownerPath, ...mirrorPaths, ...(rawContract.allowedPaths ?? [])].map(
        (entry) => entry.replaceAll("\\", "/"),
      ),
    ),
    scanRoots: rawContract.scanRoots ?? [],
    values,
  };
}

export { loadContract };
