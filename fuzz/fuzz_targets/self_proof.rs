#![no_main]

use libfuzzer_sys::fuzz_target;
use std::num::ParseIntError;
use std::str::FromStr;

use decimax::{Decimal, ParseError, UnderlyingInt};

#[derive(Debug, arbitrary::Arbitrary)]
struct Data {
    n_man128: i128,
    d_man128: i128,

    n_man64: i64,
    d_man64: i64,

    n_man32: i32,
    d_man32: i32,

    n_scale: u32,
    d_scale: u32,
}

fuzz_target!(|data: Data| {
    test_signed::<u128>(data.n_man128, data.n_scale, data.d_man128, data.d_scale);
    test_signed::<u64>(data.n_man64, data.n_scale, data.d_man64, data.d_scale);
    test_signed::<u32>(data.n_man32, data.n_scale, data.d_man32, data.d_scale);
    test_unsigned::<u128>(
        data.n_man128 as u128,
        data.n_scale,
        data.d_man128 as u128,
        data.d_scale,
    );
    test_unsigned::<u64>(
        data.n_man64 as u64,
        data.n_scale,
        data.d_man64 as u64,
        data.d_scale,
    );
    test_unsigned::<u32>(
        data.n_man32 as u32,
        data.n_scale,
        data.d_man32 as u32,
        data.d_scale,
    );
});

fn test_signed<I>(n_man: I::Signed, n_scale: u32, d_man: I::Signed, d_scale: u32) -> Option<()>
where
    I: UnderlyingInt + FromStr<Err = ParseIntError>,
{
    let n_scale = n_scale % (I::MAX_SCALE + 1);
    let d_scale = d_scale % (I::MAX_SCALE + 1);

    let n = Decimal::<I, true>::try_from_parts(n_man, n_scale)?;
    let d = Decimal::<I, true>::try_from_parts(d_man, d_scale)?;

    if let Some(q) = n.checked_div(d) {
        check_div_q_signed(n, d, q);
    }

    if n == d {
        assert!(d == n);
    } else if n > d {
        assert!(d < n);
    } else {
        assert!(d > n);
    }

    check_format(n);
    check_format(d);

    Some(())
}

fn check_div_q_signed<I>(n: Decimal<I, true>, d: Decimal<I, true>, q: Decimal<I, true>)
where
    I: UnderlyingInt,
{
    // If the @n is very big, and the @q is rounded up, then
    // the product may be overflow.
    let Some(p) = d.checked_mul(q) else {
        // dbg!(n, d, q);
        let (_, n_scale) = n.parts();
        assert!(n_scale < 1);
        return;
    };

    // It should that: @diff2 <= @d.
    // If not, there may be precision lossing in the division.
    // In this case, the @q must be very big, which means that:
    // @q_scale == 0. However since there may be some optimization,
    // then the @q_scale may be 1 too.
    let diff = (n - p).abs();
    let diff2 = diff + diff;
    if diff2 > d.abs() {
        let (_, q_scale) = q.parts();
        assert!(q_scale <= 1);
    }
}

fn test_unsigned<I>(n_man: I, n_scale: u32, d_man: I, d_scale: u32) -> Option<()>
where
    I: UnderlyingInt + FromStr<Err = ParseIntError>,
{
    let n_scale = n_scale % (I::MAX_SCALE + 1);
    let d_scale = d_scale % (I::MAX_SCALE + 1);

    let n = Decimal::<I, false>::try_from_parts(n_man, n_scale)?;
    let d = Decimal::<I, false>::try_from_parts(d_man, d_scale)?;

    if let Some(q) = n.checked_div(d) {
        check_div_q_unsigned(n, d, q);
    }

    if n == d {
        assert!(d == n);
    } else if n > d {
        assert!(d < n);
    } else {
        assert!(d > n);
    }

    check_format(n);
    check_format(d);

    Some(())
}

fn check_div_q_unsigned<I>(n: Decimal<I, false>, d: Decimal<I, false>, q: Decimal<I, false>)
where
    I: UnderlyingInt,
{
    // If the @n is very big, and the @q is rounded up, then
    // the product may be overflow.
    let Some(p) = d.checked_mul(q) else {
        // dbg!(n, d, q);
        let (_, n_scale) = n.parts();
        assert!(n_scale < 1);
        return;
    };

    // It should that: @diff2 <= @d.
    // If not, there may be precision lossing in the division.
    // In this case, the @q must be very big, which means that:
    // @q_scale == 0. However since there may be some optimization,
    // then the @q_scale may be 1 too.
    let diff = if n > p { n - p } else { p - n };
    let diff2 = diff + diff;
    if diff2 > d {
        let (_, q_scale) = q.parts();
        assert!(q_scale <= 1);
    }
}

fn check_format<I, const S: bool>(n: Decimal<I, S>)
where
    I: UnderlyingInt + FromStr<Err = ParseIntError>,
{
    let s = format!("{n}");
    assert_eq!(Decimal::<I, S>::from_str(&s).unwrap(), n);

    // set precision
    let s = format!("{:.4}", n);
    match Decimal::<I, S>::from_str(&s) {
        Ok(n2) => assert_eq!(n2, n.round_to(4)),
        Err(err) => assert_eq!(err, ParseError::Overflow),
    }
}
