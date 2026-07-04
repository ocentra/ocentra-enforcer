# PASS fixture for FSM-VALIDATEMUTATE.1: an illegal transition raises
# InvalidTransition; mutation never happens on the invalid path.


class Order:
    def assert_transition(self, target):
        if target not in self.allowed[self.status]:
            raise InvalidTransition(self.status, target)

    def transition(self, target):
        self.assert_transition(target)
        self.status = target
