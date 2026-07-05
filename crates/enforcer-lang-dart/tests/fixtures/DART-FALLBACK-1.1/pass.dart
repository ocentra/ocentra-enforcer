// PASS fixture for DART-FALLBACK-1.1: validate-then-construct, justified bang.
class OrderLine {
  final int quantity;

  factory OrderLine.fromJson(Map<String, Object?> json) {
    final raw = json['qty'];
    if (raw is! int) {
      throw FormatException('qty is required and must be an int');
    }
    return OrderLine._(raw);
  }

  OrderLine._(this.quantity);

  int readBody(Response response) {
    // data guaranteed non-null by the server contract for this endpoint.
    return response.data!;
  }
}

class Response {
  final int? data;
  const Response(this.data);
}
