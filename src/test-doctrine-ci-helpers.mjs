/** Extracts individual CI step blocks from workflow text. */
export function stepBlocks(ciText) {
  const lines = ciText.split(/\r?\n/);
  const blocks = [];
  let current = [];
  for (const line of lines) {
    if (/^\s*-\s+(name|run|uses):/.test(line) && current.length > 0) {
      blocks.push(current.join("\n"));
      current = [];
    }
    current.push(line);
  }
  if (current.length) blocks.push(current.join("\n"));
  return blocks;
}
