use lean_decimal::Dec128;

#[test]
fn cmp_base() {
    assert_eq!(Dec128::ZERO, -Dec128::ZERO);
    assert_eq!(Dec128::ZERO, Dec128::from_parts(0, 10));

    let pos = Dec128::from_parts(10, 10);
    let neg = Dec128::from_parts(-10, 10);

    assert!(Dec128::ZERO < pos);
    assert!(Dec128::ZERO > -pos);
    assert!(-Dec128::ZERO < pos);
    assert!(-Dec128::ZERO > -pos);
    assert!(Dec128::ZERO > neg);
    assert!(Dec128::ZERO < -neg);
    assert!(-Dec128::ZERO > neg);
    assert!(-Dec128::ZERO < -neg);
    assert!(pos > neg);
    assert!(-pos < -neg);
    assert!(-pos == neg);

    let a = Dec128::from_parts(100, 2);
    let b = Dec128::from_parts(300, 2);
    assert!(a < b);
    assert!(-a > -b);

    let c = Dec128::from_parts(300, 9);
    assert!(a > c);
    assert!(-a < -c);
    assert!(c < a);
    assert!(-c > -a);

    let a = Dec128::from_parts(30000, 9);
    let b = Dec128::from_parts(300, 31);
    assert!(a > b);
    assert!(b < a);
    assert!(-a < -b);
    assert!(-b > -a);
}
