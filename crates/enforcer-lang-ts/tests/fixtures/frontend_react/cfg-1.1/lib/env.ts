export const env = {
  API_URL: import.meta.env.VITE_API_URL ?? process.env.NEXT_PUBLIC_API_URL ?? "",
};
