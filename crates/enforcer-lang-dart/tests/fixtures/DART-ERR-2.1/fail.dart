// FAIL fixture for DART-ERR-2.1: raw exception rendered to the user (scored).
class ErrorView extends StatelessWidget {
  Widget build(BuildContext context, Object error) {
    return Text('$error');
  }
}
