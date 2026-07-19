/** Splits CI workflow text into executable step-shaped blocks. */
export function stepBlocks(ciText) {
  const lines = ciText.split(/\r?\n/u);
  const blocks = [];
  let current = [];
  for (const line of lines) {
    if (/^\s*-\s+(name|run|uses):/u.test(line) && current.length > 0) {
      blocks.push(current.join("\n"));
      current = [];
    }
    current.push(line);
  }
  if (current.length > 0) blocks.push(current.join("\n"));
  return blocks;
}

/** Removes prose-only YAML comments before command-pattern matching. */
export function stripCommentLines(text) {
  return text
    .split("\n")
    .filter((line) => !line.trim().startsWith("#"))
    .map((line) => line.replace(/\s#.*$/u, ""))
    .join("\n");
}
