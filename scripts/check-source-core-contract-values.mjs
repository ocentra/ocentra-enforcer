import fs from "node:fs";

import { repoAbsolute } from "../src/path-utils.mjs";
import { valueAtSourceObjectPath } from "./check-source-core-contract-object-values.mjs";
import {
  escapeRegExp,
  valueAtRustConst,
  valueAtRustSerdeRename,
} from "./check-source-core-contract-rust-values.mjs";

function valueAtPath(source, jsonPath) {
  let value = source;
  for (const segment of jsonPath.split(".")) {
    if (value === null || typeof value !== "object" || !(segment in value)) {
      throw new Error(`${jsonPath} is missing`);
    }
    value = value[segment];
  }
  return value;
}

function valueFromSpec(root, ownerPath, valueSpec) {
  const sourceText = fs.readFileSync(repoAbsolute(root, ownerPath), "utf8");
  if ("jsonPath" in valueSpec) {
    return valueAtPath(JSON.parse(sourceText), valueSpec.jsonPath);
  }
  if ("sourceObjectPath" in valueSpec) {
    return valueAtSourceObjectPath(
      sourceText,
      valueSpec.sourceObjectPath,
      ownerPath,
    );
  }
  if ("rustConst" in valueSpec) {
    return valueAtRustConst(sourceText, valueSpec.rustConst, ownerPath);
  }
  if ("rustSerdeRename" in valueSpec) {
    return valueAtRustSerdeRename(
      sourceText,
      valueSpec.rustSerdeRename,
      ownerPath,
    );
  }
  throw new Error(
    `${ownerPath}: ${valueSpec.name} needs jsonPath, sourceObjectPath, rustConst, or rustSerdeRename`,
  );
}

function createLiteralMatchPattern(value) {
  return new RegExp(
    `(?<![A-Za-z0-9@._/-])${escapeRegExp(value)}(?![A-Za-z0-9@._/-])`,
    "u",
  );
}

export {
  createLiteralMatchPattern,
  escapeRegExp,
  valueAtPath,
  valueAtRustConst,
  valueAtRustSerdeRename,
  valueAtSourceObjectPath,
  valueFromSpec,
};
