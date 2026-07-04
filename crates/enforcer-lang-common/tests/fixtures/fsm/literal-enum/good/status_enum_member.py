# PASS fixture for FSM-LITERALENUM.1: comparing a status field against a
# typed enum member, never a bare string literal.


def is_pending(order):
    if order.status == Status.PENDING:
        return True
    return False
