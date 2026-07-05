import { env } from "@/lib/env";

export function apiBaseUrl(): string {
  return env.API_URL;
}
