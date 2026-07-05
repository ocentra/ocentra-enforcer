// FAIL fixture for DART-SEC-1.5: bare print() diagnostic (scored).
class PaymentService {
  void charge(Map<String, Object?> payload) {
    print(payload);
  }
}
