#![no_main]

use libfuzzer_sys::fuzz_target;
use primitive_fixed_point_decimal::OobScaleFpdec;

type Fpdec = OobScaleFpdec<i128>;

use lean_decimal::Dec128;

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
    let n_scale = data.n_scale % 32;
    let d_scale = data.d_scale % 32;

    let n = Dec128::try_from_parts(data.n_man, n_scale)?;
    let d = Dec128::try_from_parts(data.d_man, d_scale)?;

    let n2 = Fpdec::from_mantissa(data.n_man);
    let d2 = Fpdec::from_mantissa(data.d_man);

    // mul
    if let Some(p) = n.checked_mul(d) {
        let (p_man, p_scale) = p.parts();

        let diff_scale = (n_scale + d_scale) as i32 - p_scale as i32;
        let p2 = n2.checked_mul(d2, diff_scale).unwrap();
        assert_eq!(p2.mantissa(), p_man);
    }

    // div
    if let Some(q) = n.checked_div(d) {
        let (q_man, q_scale) = q.parts();

        let diff_scale = n_scale as i32 - d_scale as i32 - q_scale as i32;
        let q2 = n2.checked_div(d2, diff_scale).unwrap();

        let q2_man = q2.mantissa();

        // the 2 crate have different rounding choice if:
        //    n < 0 && d < 0 && r*2 == q
        if q2_man != q_man {
            assert!(n.is_negative());
            assert!(d.is_negative());
            assert_eq!(q_man - q2_man, 1);
        }
    }

    Some(())
}
