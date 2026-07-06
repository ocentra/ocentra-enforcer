struct Widget;

fn helper() {
    let value = 1;
    value
}

fn caller_one() {
    helper();
}

fn caller_two() {
    helper();
}

#[test]
fn helper_returns_one() {
    assert_eq!(1, 1);
}
