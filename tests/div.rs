use lean_decimal::{Dec32, Dec64, Dec128};
use lean_decimal::{Decimal, UnderlyingInt};

fn check_div<I>(n_man: I::Signed, d_man: I::Signed, r_man: I::Signed, diff_scale: u32)
where
    I: UnderlyingInt,
{
    let n = Decimal::<I, true>::from_parts(n_man, 0);
    let d = Decimal::<I, true>::from_parts(d_man, 0);
    let r = Decimal::<I, true>::from_parts(r_man, diff_scale);
    assert_eq!(n / d, r);
}

#[test]
fn div128() {
    // 128bit: small divisor
    check_div::<u128>(12, 3, 4, 0); // even
    check_div::<u128>(12, 300, 4, 2); // even after rescale
    check_div::<u128>(12, 7, 17142857142857142857142857142857_i128, 31); // even after rescale

    // 128bit: big divisor
    let x = 10_i128.pow(12);
    check_div::<u128>(12 * x, 3 * x, 4, 0); // even
    check_div::<u128>(12 * x, 300 * x, 4, 2); // even after rescale
    check_div::<u128>(12 * x, 7 * x, 17142857142857142857142857142857_i128, 31); // even after rescale

    // overflow
    let d = Dec128::from_parts(9, 1);
    assert!(Dec128::MAX.checked_div(d).is_none());

    // by zero
    assert!(d.checked_div(Dec128::ZERO).is_none());
}

#[test]
fn div64() {
    // 64bit: small divisor
    check_div::<u64>(12, 3, 4, 0); // even
    check_div::<u64>(12, 300, 4, 2); // even after rescale
    check_div::<u64>(12, 7, 1714285714285714, 15); // even after rescale

    // 64bit: big divisor
    let x = 10_i64.pow(12);
    check_div::<u64>(12 * x, 3 * x, 4, 0); // even
    check_div::<u64>(12 * x, 300 * x, 4, 2); // even after rescale
    check_div::<u64>(12 * x, 7 * x, 1714285714285714, 15); // even after rescale

    // overflow
    let d = Dec64::from_parts(9, 1);
    assert!(Dec64::MAX.checked_div(d).is_none());

    // by zero
    assert!(d.checked_div(Dec64::ZERO).is_none());
}

#[test]
fn div32() {
    // 32bit:
    check_div::<u32>(12, 3, 4, 0); // even
    check_div::<u32>(12, 300, 4, 2); // even after rescale
    check_div::<u32>(12, 7, 17142857, 7); // even after rescale

    // overflow
    let d = Dec32::from_parts(9, 1);
    assert!(Dec32::MAX.checked_div(d).is_none());

    // by zero
    assert!(d.checked_div(Dec32::ZERO).is_none());
}
