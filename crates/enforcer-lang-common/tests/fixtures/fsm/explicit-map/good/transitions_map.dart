// PASS fixture for FSM-EXPLICITMAP.1: an explicit states->transitions map
// declared as a const, consumed by a typed transition() call.

const transitions = {
  Status.open: [Status.closed],
  Status.closed: <Status>[],
};

class Ticket {
  Status status = Status.open;

  void transition(Status next) {
    status = next;
  }
}
