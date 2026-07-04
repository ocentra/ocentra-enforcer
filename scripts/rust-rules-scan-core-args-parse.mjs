import { collectFileTokens } from "./rust-rules-scan-core-args-files.mjs";
import {
  FLAG_OPTIONS,
  VALUE_OPTIONS,
  parseFileList,
} from "./rust-rules-scan-core-args-options.mjs";

export function collectParsedTokens(tokens, args) {
  const context = {
    args,
    explicitFiles: [],
    explicitFileManifests: [],
    tokens,
  };
  for (let index = 0; index < tokens.length; index += 1) {
    index = collectParsedToken(context, index);
    if (index === tokens.length) break;
  }
  return {
    explicitFiles: context.explicitFiles,
    explicitFileManifests: context.explicitFileManifests,
  };
}

function collectParsedToken(context, index) {
  const { args, explicitFiles, explicitFileManifests, tokens } = context;
  const arg = tokens[index];
  const flag = FLAG_OPTIONS[arg];
  const valueHandler = VALUE_OPTIONS[arg];
  if (arg === "--") {
    args.runCommand = tokens.slice(index + 1);
    return tokens.length;
  }
  if (flag) {
    args[flag] = true;
    return index;
  }
  if (arg === "--files") {
    return collectFileTokens(tokens, index + 1, explicitFiles) - 1;
  }
  if (arg === "--files-from") {
    explicitFileManifests.push(tokens[index + 1]);
    return index + 1;
  }
  if (arg.startsWith("--files-from=")) {
    explicitFileManifests.push(arg.slice("--files-from=".length));
    return index;
  }
  if (valueHandler) {
    valueHandler(args, tokens[index + 1]);
    return index + 1;
  }
  if (arg.startsWith("-")) {
    throw new Error(`Unknown argument: ${arg}`);
  }
  explicitFiles.push(...parseFileList(arg));
  return index;
}
