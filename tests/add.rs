use lean_decimal::Dec128;

fn test_add(a: Dec128, b: Dec128, r: Dec128) {
    dbg!(a, b, r);
    assert_eq!(a + -b, a - b);
    assert_eq!(-a + b, b - a);
    assert_eq!(-a + -b, -(b + a));
    assert_eq!(-a + -b, -r);

    assert_eq!(a + b, r);
    assert_eq!(b + a, r);
    assert_eq!(r - a, b);
    assert_eq!(r - b, a);
}

#[test]
fn add_base() {
    // normal
    let a = Dec128::from_parts(1000, 10);
    let b = Dec128::from_parts(2000, 10);
    let c = Dec128::from_parts(3000, 10);
    test_add(a, a, b);
    test_add(a, b, c);

    // zero
    test_add(a, Dec128::ZERO, a);
    test_add(a, -Dec128::ZERO, a);

    // different scales
    let d = Dec128::from_parts(1000, 13);
    let e = Dec128::from_parts(1001000, 13);
    test_add(a, d, e);

    let max_mantissa = (1_i128 << 122) - 1;

    // different scales with big mantissa
    let big1 = Dec128::from_parts(max_mantissa - 1, 7);
    let big2 = Dec128::from_parts(max_mantissa, 7);
    test_add(a, big1, big2);

    // overflow in addition, so reduce scale
    let big1 = Dec128::from_parts(max_mantissa - 3, 10);
    let big2 = Dec128::from_parts(max_mantissa / 5, 9);
    test_add(big1, big1, big2);

    // overflow without scale to reduced
    let big1 = Dec128::from_parts(max_mantissa, 0);
    assert!(big1.checked_add(big1).is_none());
}
