import { parseFileList } from "./rust-rules-scan-core-args-options.mjs";

export function collectFileTokens(tokens, startIndex, explicitFiles) {
  let index = startIndex;
  while (index < tokens.length && !tokens[index].startsWith("-")) {
    explicitFiles.push(...parseFileList(tokens[index]));
    index += 1;
  }
  return index;
}
