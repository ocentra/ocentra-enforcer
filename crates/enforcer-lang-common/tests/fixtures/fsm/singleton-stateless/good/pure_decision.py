# PASS fixture for FSM-SINGLETONSTATELESS.1: the FSM method is a pure
# from/to -> decision function with no per-request instance state.


class OrderFsm:
    def transition(self, current, target):
        if target in self.allowed[current]:
            return target
        raise InvalidTransition(current, target)
