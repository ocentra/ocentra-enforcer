function valueAtRustConst(source, rustConst, ownerPath) {
  const constMatch = new RegExp(
    `(?:pub\\s+)?const\\s+${escapeRegExp(rustConst)}\\s*:\\s*&str\\s*=\\s*"([^"]+)"\\s*;`,
    "u",
  ).exec(source);
  if (constMatch === null) {
    throw new Error(`${ownerPath}: ${rustConst} string const is missing`);
  }
  return constMatch[1];
}

function valueAtRustSerdeRename(source, rustSerdeRename, ownerPath) {
  const segments = rustSerdeRename.split("::");
  if (
    segments.length !== 2 ||
    segments.some((segment) => segment.length === 0)
  ) {
    throw new Error(
      `${ownerPath}: ${rustSerdeRename} must be formatted as EnumName::VariantName`,
    );
  }
  const [enumName, variantName] = segments;
  const enumMatch = new RegExp(
    `enum\\s+${escapeRegExp(enumName)}\\s*\\{([\\s\\S]*?)\\n\\}`,
    "u",
  ).exec(source);
  if (enumMatch === null) {
    throw new Error(`${ownerPath}: ${enumName} enum is missing`);
  }
  const variantMatch = new RegExp(
    `#\\[serde\\(rename\\s*=\\s*"([^"]+)"\\)\\]\\s*${escapeRegExp(variantName)}\\b`,
    "u",
  ).exec(enumMatch[1]);
  if (variantMatch === null) {
    throw new Error(`${ownerPath}: ${rustSerdeRename} serde rename is missing`);
  }
  return variantMatch[1];
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

export { escapeRegExp, valueAtRustConst, valueAtRustSerdeRename };
