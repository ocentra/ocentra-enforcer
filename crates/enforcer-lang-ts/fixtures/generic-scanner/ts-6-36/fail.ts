export function toWidget(raw: unknown): Widget {
  return { ...(raw as any) };
}
