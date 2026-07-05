// PASS fixture for DART-STATE-1.1: Riverpod Notifier used instead.
class OrderCountNotifier extends Notifier<int> {
  int build() => 0;

  void increment() {
    state += 1;
  }
}
