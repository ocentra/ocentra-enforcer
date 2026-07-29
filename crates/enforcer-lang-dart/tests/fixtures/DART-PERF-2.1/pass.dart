// PASS fixture for DART-PERF-2.1: setState() only called from an event handler,
// never from inside the widget's build method itself.
class Counter extends State<CounterWidget> {
  int count = 0;

  void increment() {
    setState(() {
      count += 1;
    });
  }

  Widget build(BuildContext context) {
    return ElevatedButton(
      onPressed: increment,
      child: Text('$count'),
    );
  }
}
