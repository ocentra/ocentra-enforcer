// FAIL fixture for DART-PERF-2.1: setState() called inside build().
class Counter extends State<CounterWidget> {
  int count = 0;

  Widget build(BuildContext context) {
    setState(() => count += 1);
    return Text('$count');
  }
}
