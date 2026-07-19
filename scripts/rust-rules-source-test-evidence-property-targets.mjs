export function propertyEvidenceTargets(sources, propertyBodies, registeredBodies) {
  const propertyTargetNames = new Set();
  const registeredPropertyTargets = new Set();
  for (const bodies of propertyBodies) {
    for (const body of bodies) {
      for (const match of body.matchAll(/\b[A-Za-z_][A-Za-z0-9_]*\b/gu)) propertyTargetNames.add(match[0]);
    }
  }
  sources.forEach((evidenceSource, index) => {
    if (!/\bproptest!\s*\{/u.test(evidenceSource)) return;
    for (const body of registeredBodies[index]) {
      for (const match of body.matchAll(/["']([^"']+)["']\s*=>/gu)) registeredPropertyTargets.add(match[1]);
    }
  });
  return { propertyTargetNames, registeredPropertyTargets };
}
