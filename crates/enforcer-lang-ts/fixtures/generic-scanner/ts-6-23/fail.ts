export function fireAndForget(): void {
  saveWidget().catch(() => {});
}
