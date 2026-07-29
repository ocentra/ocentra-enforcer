// PASS fixture for DART-INITSTATE-1.1: provider/FutureBuilder drives the fetch.
class OrderListPage extends ConsumerWidget {
  Widget build(BuildContext context, WidgetRef ref) {
    final orders = ref.watch(orderListProvider);
    return orders.when(
      data: (data) => ListView.builder(itemCount: data.length, itemBuilder: (c, i) => Text('x')),
      loading: () => const CircularProgressIndicator(),
      error: (e, st) => const Text('error'),
    );
  }
}
