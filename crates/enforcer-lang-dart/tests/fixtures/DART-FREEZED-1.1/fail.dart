// FAIL fixture for DART-FREEZED-1.1: mutable entity with a setter.
class Ticket {
  String _status = 'open';

  String get status => _status;
  set status(String value) {
    _status = value;
  }
}
