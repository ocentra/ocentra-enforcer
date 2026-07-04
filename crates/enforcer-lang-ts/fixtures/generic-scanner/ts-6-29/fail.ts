export function patchWidget(patch: Partial<Widget>): Widget {
  return { ...widget, ...patch };
}
