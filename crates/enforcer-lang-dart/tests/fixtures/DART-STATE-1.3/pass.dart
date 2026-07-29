// PASS fixture for DART-STATE-1.3: emits an event and navigates back with a result.
class OrderDetailPage extends ConsumerWidget {
  void onSave(BuildContext context, Order order) {
    Navigator.of(context).pop(order);
  }
}
