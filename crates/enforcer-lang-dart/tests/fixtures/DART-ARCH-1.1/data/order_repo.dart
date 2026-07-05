// FAIL fixture for DART-ARCH-1.1: a data/ file reaches into presentation/.
import 'package:app/domain/order.dart';
import '../presentation/order_page.dart';

class OrderRepository {
  Future<void> save(Order order) async {}
}
