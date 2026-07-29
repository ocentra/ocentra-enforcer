// PASS fixture for DART-ERR-2.1: generic user-facing message shown instead.
class ErrorView extends StatelessWidget {
  Widget build(BuildContext context, Object error) {
    logger.error('unexpected error', error);
    return const Text('Something went wrong. Please try again.');
  }
}
