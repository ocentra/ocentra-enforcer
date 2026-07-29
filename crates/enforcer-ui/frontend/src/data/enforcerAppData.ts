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

// The shipped desktop inventory starts EMPTY: every connected project comes
// from the desktop-local registry (user registration or Git worktree
// discovery), never from roots or statuses invented at build time.
export const appData: { projects: Project[] } = {
  projects: [],
};

/** Rust-owned count scalar rendered by the presentation layer. */
export type UiCount = number;

/** Rust-owned boolean state rendered by the presentation layer. */
export type UiFlag = boolean;

/** Rust-owned text collection rendered by the presentation layer. */
export type UiTextList = ReadonlyArray<string>;

/** Rust-owned nullable transport field rendered by the presentation layer. */
export type UiMaybe<T> = T | undefined;
