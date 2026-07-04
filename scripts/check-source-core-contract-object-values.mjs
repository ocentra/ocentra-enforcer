import { escapeRegExp } from "./check-source-core-contract-rust-values.mjs";

function valueAtSourceObjectPath(source, sourceObjectPath, ownerPath) {
  const lastDotIndex = sourceObjectPath.lastIndexOf(".");
  if (lastDotIndex <= 0 || lastDotIndex === sourceObjectPath.length - 1) {
    throw new Error(
      `${ownerPath}: ${sourceObjectPath} must be formatted as ObjectName.PropertyName or ObjectName.PropertyName[index]`,
    );
  }
  const objectName = sourceObjectPath.slice(0, lastDotIndex);
  const propertyPath = sourceObjectPath.slice(lastDotIndex + 1);
  const arrayIndexMatch =
    /^(?<propertyName>[A-Za-z0-9_]+)\[(?<index>\d+)\]$/u.exec(propertyPath);
  const propertyName = arrayIndexMatch?.groups?.propertyName ?? propertyPath;
  const objectBody = objectBodyFor(source, objectName);
  const directString = directObjectString(objectBody, propertyName);
  if (directString !== null) return directString;
  const parsedString = parsedObjectString(objectBody, propertyName);
  if (parsedString !== null) return parsedString;
  if (arrayIndexMatch === null) {
    throw new Error(
      `${ownerPath}: ${sourceObjectPath} string literal is missing`,
    );
  }
  return arrayObjectString(objectBody, propertyName, arrayIndexMatch, ownerPath, sourceObjectPath);
}

function objectBodyFor(source, objectName) {
  const objectPattern = new RegExp(
    `(?:export\\s+)?const\\s+${escapeRegExp(objectName)}\\s*=\\s*\\{([\\s\\S]*?)\\}\\s*(?:as\\s+const)?`,
    "u",
  );
  const kindGroupPattern = new RegExp(
    `(?:export\\s+)?const\\s+${escapeRegExp(objectName)}\\s*=\\s*defineLiteralKindGroup\\(\\s*\\{([\\s\\S]*?)\\}\\s*(?:as\\s+const)?\\s*\\)`,
    "u",
  );
  return objectPattern.exec(source)?.[1] ?? kindGroupPattern.exec(source)?.[1];
}

function directObjectString(objectBody, propertyName) {
  if (objectBody === undefined) return null;
  return new RegExp(
    `\\b${escapeRegExp(propertyName)}\\s*:\\s*(['"\`])([^'"\`]+)\\1`,
    "u",
  ).exec(objectBody)?.[2] ?? null;
}

function parsedObjectString(objectBody, propertyName) {
  if (objectBody === undefined) return null;
  return new RegExp(
    `\\b${escapeRegExp(propertyName)}\\s*:\\s*[A-Za-z0-9_$.]+\\.parse\\(\\s*(['"\`])([^'"\`]+)\\1\\s*\\)`,
    "u",
  ).exec(objectBody)?.[2] ?? null;
}

function arrayObjectString(
  objectBody,
  propertyName,
  arrayIndexMatch,
  ownerPath,
  sourceObjectPath,
) {
  if (objectBody === undefined) {
    throw new Error(`${ownerPath}: ${sourceObjectPath} constant object is missing`);
  }
  const arrayMatch = new RegExp(
    `\\b${escapeRegExp(propertyName)}\\s*:\\s*\\[([\\s\\S]*?)\\]`,
    "u",
  ).exec(objectBody);
  if (arrayMatch === null) {
    throw new Error(`${ownerPath}: ${sourceObjectPath} array literal is missing`);
  }
  const stringMatches = [...arrayMatch[1].matchAll(/(['"`])([^'"`]+)\1/gu)];
  const index = Number.parseInt(arrayIndexMatch.groups.index, 10);
  if (index < stringMatches.length) return stringMatches[index][2];
  throw new Error(`${ownerPath}: ${sourceObjectPath} array entry is missing`);
}

export { valueAtSourceObjectPath };
