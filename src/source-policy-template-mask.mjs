function hasEvenTrailingBackslashes(line, index) {
  let slashCount = 0;
  for (let cursor = index - 1; cursor >= 0 && line[cursor] === '\\'; cursor -= 1) {
    slashCount += 1;
  }
  return slashCount % 2 === 0;
}

function findUnescapedBacktick(line, start) {
  for (let index = start; index < line.length; index += 1) {
    if (line[index] === '`' && hasEvenTrailingBackslashes(line, index)) return index;
  }
  return -1;
}

function maskTemplateTail(line, state, index) {
  const end = findUnescapedBacktick(line, index);
  if (end < 0) return { next: line.length, text: '' };
  state.inTemplate = false;
  return { next: end + 1, text: '``' };
}

function maskCodeSegment(line, state, index, maskSegment) {
  const next = findUnescapedBacktick(line, index);
  if (next < 0) return { next: line.length, text: maskSegment(line.slice(index)) };
  state.inTemplate = true;
  return { next: next + 1, text: `${maskSegment(line.slice(index, next))}\`\`` };
}

function maskLineWithTemplateState(line, state, maskSegment) {
  let output = '';
  let index = 0;
  while (index < line.length) {
    const result = state.inTemplate
      ? maskTemplateTail(line, state, index)
      : maskCodeSegment(line, state, index, maskSegment);
    output += result.text;
    index = result.next;
  }
  return output;
}

export function maskMultilineTemplates(lines, maskSegment) {
  const state = { inTemplate: false };
  return lines.map((line) =>
    maskLineWithTemplateState(String(line ?? ''), state, maskSegment),
  );
}
