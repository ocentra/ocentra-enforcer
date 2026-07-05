// PASS fixture for DART-SEC-1.5: routed through a monitoring logger.
class PaymentService {
  void charge(Map<String, Object?> payload) {
    logger.info('charging payload', payload);
  }
}
