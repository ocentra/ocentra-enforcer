// FAIL fixture for DART-BANG-1.1: unchecked null-assertion on a map lookup.
class OrderPage {
  void load(Map<String, String> pathParameters) {
    final id = int.parse(pathParameters['id']!);
    print(id);
  }
}
