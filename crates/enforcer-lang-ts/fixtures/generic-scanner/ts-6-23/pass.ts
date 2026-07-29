export function fireAndForget(): void {
  saveWidget().catch((error) => logger.error(error));
}
