// PASS fixture for DART-ERR-1.1: typed sealed Failure thrown.
sealed class Failure {}

class ServerFailure extends Failure {
  final String message;
  ServerFailure(this.message);
}

class OrderRepository {
  void save() {
    throw ServerFailure('boom');
  }
}
