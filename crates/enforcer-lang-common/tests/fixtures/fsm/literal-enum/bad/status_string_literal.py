# FAIL fixture for FSM-LITERALENUM.1: comparing a status field against a
# bare string literal instead of an enum member.


def is_pending(order):
    if order.status == "pending":
        return True
    return False
