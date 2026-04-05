use lean_decimal::Decimal;

type Dec = Decimal<u128>;

#[test]
fn div_base() {
    // divide by 0
    let a = Dec::from_parts(100, 2);
    assert!(a.checked_div(Dec::ZERO).is_none());
}
