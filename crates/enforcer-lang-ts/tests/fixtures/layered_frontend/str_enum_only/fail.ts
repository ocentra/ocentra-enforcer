// FRONT-04 fail fixture: an implicit-value (numeric) enum member with no
// string-literal initializer.
export enum OrderStatus {
  Pending = "pending",
  Shipped,
  Delivered = "delivered",
}
