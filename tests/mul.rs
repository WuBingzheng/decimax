use lean_decimal::Decimal;

type Dec = Decimal<u128>;

#[test]
fn mul_base() {
    // normal
    let a = Dec::from_parts(100, 2);
    let b = Dec::from_parts(300, 3);
    let r = Dec::from_parts(30000, 5);
    assert_eq!(a * b, r);

    // sum of scale overflow
    let a = Dec::from_parts(100, 12);
    let b = Dec::from_parts(300, 26);
    let r = Dec::from_parts(300, 36);
    assert_eq!(a * b, r);

    // product of mantissas overflow of mantissa(121 bit) but
    // fits in u128 (128 bit)
    let a = Dec::from_parts(1_i128 << 60, 6);
    let b = Dec::from_parts(1_i128 << 63, 8);
    let r = Dec::from_parts(((1_i128 << 123) + 5) / 10, 6 + 8 - 1);
    assert_eq!(a * b, r);

    // product of mantissas overflow
    let a = Dec::from_parts(1_i128 << 90, 12);
    let b = Dec::from_parts(1_i128 << 90, 14);
    let r = Dec::from_parts(153249554086588885835834702715030918_i128, 12 + 14 - 19);
    assert_eq!(a * b, r);

    // overflow
    let a = Dec::from_parts(1_i128 << 90, 12);
    let b = Dec::from_parts(1_i128 << 90, 4);
    assert!(a.checked_mul(b).is_none());
}
