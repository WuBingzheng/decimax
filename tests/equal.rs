use lean_decimal::Decimal;

type Dec = Decimal<u128>;

#[test]
fn equal_base() {
    assert_eq!(Dec::ZERO, -Dec::ZERO);
    assert_eq!(Dec::ZERO, Dec::from_parts(0, 10));

    let n = Dec::from_parts(100, 10);
    assert_ne!(Dec::ZERO, n);
    assert_ne!(n, Dec::ZERO);
    assert_ne!(Dec::ZERO, -n);
    assert_ne!(-n, -Dec::ZERO);

    let neg = Dec::from_parts(-100, 10);
    assert_ne!(neg, n);
    assert_ne!(-neg, -n);
    assert_eq!(-neg, n);
    assert_eq!(neg, -n);

    let a = Dec::from_parts(100, 10);
    let b = Dec::from_parts(10000, 12);
    assert_eq!(a, b);
    assert_ne!(a, -b);

    let a = Dec::from_parts(100, 35);
    let b = Dec::from_parts(10000, 12);
    assert_ne!(a, b);
    assert_ne!(a, -b);
}
