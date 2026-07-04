export type Widget =
  | { readonly kind: "named"; readonly nickname: string }
  | { readonly kind: "unnamed" };
