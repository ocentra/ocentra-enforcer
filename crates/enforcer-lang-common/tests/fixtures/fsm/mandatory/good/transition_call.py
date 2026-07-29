# PASS fixture for FSM-1.1: the `status` mutation routes through a
# transition call rather than a raw assignment.


class Order:
    def ship(self):
        fsm.assert_transition(self.status, Target.SHIPPED)
        self.status = fsm.transition(self.status, Target.SHIPPED)
