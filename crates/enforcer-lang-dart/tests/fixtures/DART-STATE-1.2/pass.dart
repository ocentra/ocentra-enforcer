// PASS fixture for DART-STATE-1.2: ref.watch used inside build().
class OrderPage extends ConsumerWidget {
  Widget build(BuildContext context, WidgetRef ref) {
    final count = ref.watch(orderCountProvider);
    return Text('$count');
  }
}
