const word = (...parts) => parts.join('');
const doubleTerms = {
  m: word('mo', 'ck'),
  f: word('fa', 'ke'),
  s: word('st', 'ub'),
  p: word('sp', 'y'),
  po: word('sp', 'y', 'On'),
};

export const testDoublePatterns = [
  { label: 'module double API', pattern: new RegExp(String.raw`\b(?:vi|jest)\.${doubleTerms.m}\b`, 'iu') },
  { label: 'double function API', pattern: /\b(?:vi|jest)\.fn\b/iu },
  {
    label: 'observer double API',
    pattern: new RegExp(String.raw`\b(?:vi|jest)\.${doubleTerms.po}\b|\b${doubleTerms.po}\b`, 'iu'),
  },
  {
    label: 'test-double package',
    pattern: new RegExp(String.raw`\b(?:${word('si', 'non')}|${word('no', 'ck')}|${word('m', 'sw')})\b`, 'iu'),
  },
  {
    label: 'test-double vocabulary',
    pattern: new RegExp(String.raw`\b(?:${doubleTerms.m}|${doubleTerms.f}|${doubleTerms.s}|${doubleTerms.p})\b`, 'iu'),
  },
];
