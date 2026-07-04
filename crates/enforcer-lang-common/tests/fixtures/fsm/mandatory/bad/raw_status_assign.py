# FAIL fixture for FSM-1.1: a `status` field mutated via raw assignment
# instead of routing through a transition call.


class Order:
    def ship(self):
        self.status = "shipped"
