export function apiBaseUrl(): string {
  return import.meta.env.VITE_API_URL ?? process.env.NEXT_PUBLIC_API_URL ?? "";
}
