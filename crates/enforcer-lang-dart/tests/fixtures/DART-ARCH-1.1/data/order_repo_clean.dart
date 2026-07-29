// PASS fixture for DART-ARCH-1.1: a data/ file imports only domain/core.
import 'package:app/domain/order.dart';
import 'package:app/core/network_client.dart';

class OrderRepository {
  Future<void> save(Order order) async {}
}
