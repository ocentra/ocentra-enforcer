// FAIL fixture for DART-FALLBACK-1.1: silent default fallback on parse.
class OrderLine {
  final int quantity;

  OrderLine.fromJson(Map<String, Object?> json)
      : quantity = json['qty'] as int? ?? 0;
}
