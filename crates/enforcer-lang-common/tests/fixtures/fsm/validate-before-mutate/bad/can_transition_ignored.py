# FAIL fixture for FSM-VALIDATEMUTATE.1: canTransition returns a bool but
# the mutation happens regardless of the result.


class Order:
    def can_transition(self, target):
        return target in self.allowed[self.status]

    def transition(self, target):
        self.can_transition(target)
        self.status = target
