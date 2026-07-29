// PASS fixture for DART-L10N-2.1: text routed through l10n.
class SubmitButton extends StatelessWidget {
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    Text(l10n.submitOrder);
    return const Placeholder();
  }
}
