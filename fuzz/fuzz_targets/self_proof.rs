#![no_main]

use libfuzzer_sys::fuzz_target;

use lean_decimal::Decimal;

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
    let n = Dec128::try_from_parts(data.n_man, data.n_scale % 36)?;
    let d = Dec128::try_from_parts(data.d_man, data.d_scale % 36)?;

    let q = n.checked_div(d)?;
    check_div_q(n, d, q);

    let p = n.checked_mul(d)?;
    check_mul_p(n, d, p);
    check_mul_p(d, n, p);

    if n == d {
        assert!(d == n);
    } else if n > d {
        assert!(d < n);
    } else {
        assert!(d > n);
    }

    Some(())
}

fn check_div_q(n: Dec128, d: Dec128, q: Dec128) {
    // If the @n is very big, and the @q is rounded up, then
    // the product may be overflow.
    let Some(p) = d.checked_mul(q) else {
        let (n_man, _) = n.parts();
        assert!(n_man.abs() >= (1_i128 << 121) - 10);
        return;
    };

    // It should that: @diff2 <= @d.
    // If not, there may be precision lossing in the division.
    // In this case, the @q must be very big, which means that:
    // @q_scale == 0 and @q_man is big. However since there may
    // be some optimization, the @q_scale may be 1 and the
    // @q_man may be not that big.
    let diff = (n - p).abs();
    let diff2 = diff.checked_mul_int(2).unwrap();
    if diff2 > d.abs() {
        let (q_man, q_scale) = q.parts();
        let (d_man, _) = d.parts();
        let exp = if d_man <= u32::MAX as i128 { 29 } else { 34 };
        assert!(q_man.abs() >= 10_i128.pow(exp));
        assert!(q_scale < 2);
    }
}

fn check_mul_p(a: Dec128, b: Dec128, p: Dec128) {
    if b.is_zero() {
        assert!(p.is_zero());
        return;
    }

    let Some(q) = p.checked_div(b) else {
        let (a_man, _) = a.parts();
        assert!(a_man.abs() >= (1_i128 << 121) - 10);
        return;
    };

    let diff = q - a;

    //          1
    // ------------------- > diff
    // b * 10.pow(p.scale)
    //
    // diff * b * 10.pow(p.scale) < 1

    let (diff_man, diff_scale) = diff.parts();
    let (b_man, b_scale) = b.parts();
    let (_, p_scale) = p.parts();

    let iexp = diff_scale + b_scale - p_scale + 2;
    if iexp < 39 {
        let x = (diff_man * b_man).abs();
        assert!(x <= 10_i128.pow(iexp));
    }
}
