export class OrderViewModel {
  status = "pending";

  ship(): void {
    this.status = "shipped";
  }
}
