# FAIL fixture for FSM-SINGLETONSTATELESS.1: an FSM class stores
# per-request instance state inside a transition method instead of staying
# a pure from/to -> decision function.


class OrderFsm:
    def transition(self, order, target):
        self.variables = {"order": order}
        return target
