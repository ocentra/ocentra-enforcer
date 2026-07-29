use starknet::ContractAddress;

#[starknet::contract]
mod Counter {
    #[storage]
    struct Storage {
        value: u128,
    }

    #[external(v0)]
    fn increment(ref self: ContractState) {
        let current = self.value.read();
        self.value.write(current + 1);
    }

    fn helper(x: u128) -> u128 {
        if x > 0 {
            x + 1
        } else {
            0
        }
    }
}
