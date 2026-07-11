export type Project = {
  id: string;
  name: string;
  root: string;
  repoKey: string;
  kind: "main" | "worktree" | "external";
  mainRoot?: string;
  branch: string;
  worktree: string;
  indexed: "ready" | "stale" | "missing";
  inspection?: "live" | "unavailable" | "configured";
  detectedLanguages: string[];
};

export function summarizeLanguages(languages: readonly string[], limit = 3): string {
  const unique = [...new Set(languages)];
  if (unique.length === 0) return "no language observed";
  const visible = unique.slice(0, limit);
  const remaining = unique.length - visible.length;
  return remaining > 0 ? `${visible.join(" / ")} +${remaining}` : visible.join(" / ");
}

// Bundled roots seed the desktop inventory. User registrations are persisted locally.
// Live inspection replaces branch and language labels when each root is reachable.
export const appData = {
  projects: [
    {
      id: "x06-desktop-fixture",
      name: "X06 Desktop Fixture",
      root: "E:/ocentra-enforcer-rust-build/crates/enforcer-memory/tests/fixtures/memory/feature_parity/repo",
      repoKey: "x06-desktop-fixture",
      kind: "external",
      branch: "fixture",
      worktree: "controlled",
      indexed: "missing",
      detectedLanguages: ["rust"],
    },
    {
      id: "enforcer",
      name: "Ocentra Enforcer",
      root: "E:/ocentra-enforcer-rust-build",
      repoKey: "ocentra-enforcer",
      kind: "worktree",
      mainRoot: "E:/ocentra-enforcer",
      branch: "rust-build",
      worktree: "primary",
      indexed: "missing",
      detectedLanguages: ["rust", "typescript"],
    },
    {
      id: "parent",
      name: "Ocentra Parent",
      root: "E:/OcentraParent",
      repoKey: "ocentra-parent",
      kind: "main",
      branch: "main",
      worktree: "prod",
      indexed: "stale",
      detectedLanguages: ["typescript", "rust", "python"],
    },
    {
      id: "games",
      name: "Ocentra Games",
      root: "E:/ocentra-games",
      repoKey: "ocentra-games",
      kind: "main",
      branch: "dev",
      worktree: "user-owned",
      indexed: "missing",
      detectedLanguages: ["typescript"],
    },
  ] satisfies Project[],
};
