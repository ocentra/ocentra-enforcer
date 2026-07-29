// PASS fixture for DART-BANG-1.1: guarded parse, guarded cast.
class OrderPage {
  void load(Map<String, String> pathParameters) {
    final raw = pathParameters['id'];
    final id = raw == null ? null : int.tryParse(raw);
    print(id);
  }

  void handle(Object x) {
    if (x is Order) {
      final o = x;
      print(o);
    }
  }
}

class Order {}
