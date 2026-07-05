// PASS fixture for DART-FORMMAP-1.1: typed form-state class.
class OrderWizardState {
  final String? sku;
  final int? quantity;

  const OrderWizardState({this.sku, this.quantity});

  OrderWizardState copyWith({String? sku, int? quantity}) {
    return OrderWizardState(
      sku: sku ?? this.sku,
      quantity: quantity ?? this.quantity,
    );
  }
}
