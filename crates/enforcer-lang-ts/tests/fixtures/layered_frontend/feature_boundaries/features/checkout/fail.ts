// FRONT-03 fail fixture: a `features/checkout/**` module deep-importing
// another feature's internals via a relative path.
import { InventoryLock } from "../otherFeature/internal/inventory-lock";

export function reserveInventory(orderId: string): InventoryLock {
  return new InventoryLock(orderId);
}
