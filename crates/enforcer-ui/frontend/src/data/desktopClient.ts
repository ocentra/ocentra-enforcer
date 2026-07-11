import { invoke } from "@tauri-apps/api/core";

/** Keeps native command invocation out of presentation components. */
export function invokeDesktop<T>(command: string, args?: Parameters<typeof invoke>[1]): Promise<T> {
  return invoke<T>(command, args);
}
