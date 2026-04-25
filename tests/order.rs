use lean_decimal::Decimal;

type Dec = Decimal<u128>;

#[test]
fn cmp_base() {
    assert_eq!(Dec::ZERO, -Dec::ZERO);
    assert_eq!(Dec::ZERO, Dec::from_parts(0, 10));

    let pos = Dec::from_parts(10, 10);
    let neg = Dec::from_parts(-10, 10);

    assert!(Dec::ZERO < pos);
    assert!(Dec::ZERO > -pos);
    assert!(-Dec::ZERO < pos);
    assert!(-Dec::ZERO > -pos);
    assert!(Dec::ZERO > neg);
    assert!(Dec::ZERO < -neg);
    assert!(-Dec::ZERO > neg);
    assert!(-Dec::ZERO < -neg);
    assert!(pos > neg);
    assert!(-pos < -neg);
    assert!(-pos == neg);

    let a = Dec::from_parts(100, 2);
    let b = Dec::from_parts(300, 2);
    assert!(a < b);
    assert!(-a > -b);

    let c = Dec::from_parts(300, 9);
    assert!(a > c);
    assert!(-a < -c);
    assert!(c < a);
    assert!(-c > -a);

    let a = Dec::from_parts(30000, 9);
    let b = Dec::from_parts(300, 31);
    assert!(a > b);
    assert!(b < a);
    assert!(-a < -b);
    assert!(-b > -a);
}
