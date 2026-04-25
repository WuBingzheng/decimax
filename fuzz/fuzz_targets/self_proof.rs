#![no_main]

use libfuzzer_sys::fuzz_target;
use std::str::FromStr;

use lean_decimal::{Decimal, ParseError};

type Dec128 = Decimal<u128>;

#[derive(Debug, arbitrary::Arbitrary)]
struct Data {
    n_man: i128,
    n_scale: u32,
    d_man: i128,
    d_scale: u32,
}

fuzz_target!(|data: Data| {
    do_test(data);
});

fn do_test(data: Data) -> Option<()> {
    let n = Dec128::try_from_parts(data.n_man, data.n_scale % 32)?;
    let d = Dec128::try_from_parts(data.d_man, data.d_scale % 32)?;

    if let Some(q) = n.checked_div(d) {
        check_div_q(n, d, q);
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

fn check_div_q(n: Dec128, d: Dec128, q: Dec128) {
    // If the @n is very big, and the @q is rounded up, then
    // the product may be overflow.
    let Some(p) = d.checked_mul(q) else {
        let (n_man, _) = n.parts();
        assert!(n_man.abs() >= (1_i128 << 122) - 10);
        return;
    };

    // It should that: @diff2 <= @d.
    // If not, there may be precision lossing in the division.
    // In this case, the @q must be very big, which means that:
    // @q_scale == 0. However since there may be some optimization,
    // then the @q_scale may be 1 too.
    let diff = (n - p).abs();
    let diff2 = diff * 2;
    if diff2 > d.abs() {
        let (_, q_scale) = q.parts();
        assert!(q_scale <= 1);
    }
}

fn check_format(n: Dec128) {
    let s = format!("{n}");
    assert_eq!(Dec128::from_str(&s).unwrap(), n);

    // set precision
    let s = format!("{:.4}", n);
    match Dec128::from_str(&s) {
        Ok(n2) => assert_eq!(n2, n.round_to(4)),
        Err(err) => assert_eq!(err, ParseError::Overflow),
    }
}
