// Catfood onboarding fixture -- clean baseline. The onboarding test's
// "seeded violation" case is layered on top of this file's content by the
// test itself (it writes a known-bad line, runs the gate, then removes it
// and re-runs) -- this fixture file's checked-in state is always clean.
export function greet(name: string): string {
  return `hello, ${name}`;
}
