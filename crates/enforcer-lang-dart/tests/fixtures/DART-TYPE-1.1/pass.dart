// PASS fixture for DART-TYPE-1.1: typed nested DTO class.
class OrderLine {
  final String sku;
  final int quantity;

  const OrderLine({required this.sku, required this.quantity});
}

class OrderDto {
  final String id;
  final List<OrderLine> lines;

  const OrderDto({required this.id, required this.lines});

  factory OrderDto.fromJson(Map<String, Object?> json) {
    return OrderDto(id: json['id'] as String, lines: const []);
  }
}
