export function parsePayload(raw: unknown): unknown {
  return JSON.parse(raw as string);
}

// waiver: any // reason: third-party SDK callback has no upstream types (TICKET-123)
export function legacySdkCallback(payload: any): void {
  console.log(payload);
}
