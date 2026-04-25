use lean_decimal::Dec128;

#[test]
fn equal_base() {
    assert_eq!(Dec128::ZERO, -Dec128::ZERO);
    assert_eq!(Dec128::ZERO, Dec128::from_parts(0, 10));

    let n = Dec128::from_parts(100, 10);
    assert_ne!(Dec128::ZERO, n);
    assert_ne!(n, Dec128::ZERO);
    assert_ne!(Dec128::ZERO, -n);
    assert_ne!(-n, -Dec128::ZERO);

    let neg = Dec128::from_parts(-100, 10);
    assert_ne!(neg, n);
    assert_ne!(-neg, -n);
    assert_eq!(-neg, n);
    assert_eq!(neg, -n);

    let a = Dec128::from_parts(100, 10);
    let b = Dec128::from_parts(10000, 12);
    assert_eq!(a, b);
    assert_ne!(a, -b);

    let a = Dec128::from_parts(100, 31);
    let b = Dec128::from_parts(10000, 12);
    assert_ne!(a, b);
    assert_ne!(a, -b);
}
