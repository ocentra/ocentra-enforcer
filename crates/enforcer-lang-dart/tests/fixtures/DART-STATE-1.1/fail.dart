// FAIL fixture for DART-STATE-1.1: ChangeNotifier used in new code.
class OrderController extends ChangeNotifier {
  int _count = 0;
  int get count => _count;

  void increment() {
    _count += 1;
    notifyListeners();
  }
}
