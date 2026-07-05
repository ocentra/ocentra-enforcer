// FAIL fixture for DART-FORMMAP-1.1: form state as an untyped map.
class OrderWizard {
  Map<String, Object?> formData = {};

  void setField(String key, Object? value) {
    formData[key] = value;
  }
}
