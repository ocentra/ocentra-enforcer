// FAIL fixture for DART-STATE-1.2: ref.read used inside build().
class OrderPage extends ConsumerWidget {
  Widget build(BuildContext context, WidgetRef ref) {
    final count = ref.read(orderCountProvider);
    return Text('$count');
  }
}
