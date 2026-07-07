module 0x1::counter {
    use std::signer;

    struct Counter has key {
        value: u64,
    }

    public fun initialize(account: &signer) {
        move_to(account, Counter { value: 0 });
    }

    public fun increment(account: &signer) {
        let addr = signer::address_of(account);
        let counter = borrow_global_mut<Counter>(addr);
        counter.value = counter.value + 1;
        if (counter.value > 100) {
            counter.value = 0;
        };
    }
}
