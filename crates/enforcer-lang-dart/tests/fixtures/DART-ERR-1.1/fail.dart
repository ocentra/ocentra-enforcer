// FAIL fixture for DART-ERR-1.1: raw Exception thrown instead of a typed Failure (scored).
class OrderRepository {
  void save() {
    throw Exception('boom');
  }
}
