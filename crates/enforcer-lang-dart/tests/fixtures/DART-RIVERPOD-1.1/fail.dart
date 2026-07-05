// FAIL fixture for DART-RIVERPOD-1.1: legacy StateNotifierProvider used.
final orderCountProvider =
    StateNotifierProvider<OrderCountNotifier, int>((ref) => OrderCountNotifier());
