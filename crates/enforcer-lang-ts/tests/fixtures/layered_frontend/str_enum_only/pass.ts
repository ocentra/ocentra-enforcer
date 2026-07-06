// FRONT-04 pass fixture: every enum member carries a string-literal
// initializer.
export enum OrderStatus {
  Pending = "pending",
  Shipped = "shipped",
  Delivered = "delivered",
}
