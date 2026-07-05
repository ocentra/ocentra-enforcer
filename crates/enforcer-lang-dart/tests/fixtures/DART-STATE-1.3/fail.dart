// FAIL fixture for DART-STATE-1.3: detail page mutates a list provider (scored).
class OrderDetailPage extends ConsumerWidget {
  void onSave(WidgetRef ref, Order order) {
    ref.read(listProvider.notifier).update(order);
  }
}
