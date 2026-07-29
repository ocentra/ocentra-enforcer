// FAIL fixture for FSM-EXPLICITMAP.1: no declared states->transitions map;
// mutation goes through an ad-hoc `setStatus(String)` taking a bare string.

class Ticket {
  String status = "open";

  void setStatus(String next) {
    status = next;
  }
}
