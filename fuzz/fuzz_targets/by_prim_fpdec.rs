#![no_main]

use libfuzzer_sys::fuzz_target;
use primitive_fixed_point_decimal::{FpdecInner, OobScaleFpdec};

use decimax::{Decimal, UnderlyingInt};

#[derive(Debug, arbitrary::Arbitrary)]
struct Data {
    n_man128: i128,
    d_man128: i128,

    d_man64: i64,
    n_man64: i64,

    d_man32: i32,
    n_man32: i32,

    n_scale: u32,
    d_scale: u32,
}

fuzz_target!(|data: Data| {
    test_signed::<u128, _>(data.n_man128, data.n_scale, data.d_man128, data.d_scale);
    test_signed::<u64, _>(data.n_man64, data.n_scale, data.d_man64, data.d_scale);
    test_signed::<u32, _>(data.n_man32, data.n_scale, data.d_man32, data.d_scale);

    test_unsigned(
        data.n_man128 as u128,
        data.n_scale,
        data.d_man128 as u128,
        data.d_scale,
    );
    test_unsigned(
        data.n_man64 as u64,
        data.n_scale,
        data.d_man64 as u64,
        data.d_scale,
    );
    test_unsigned(
        data.n_man32 as u32,
        data.n_scale,
        data.d_man32 as u32,
        data.d_scale,
    );
});

fn test_signed<UI, I>(n_man: I, n_scale: u32, d_man: I, d_scale: u32) -> Option<()>
where
    I: FpdecInner + std::fmt::Debug,
    UI: UnderlyingInt<Signed = I>,
{
    let n_scale = n_scale % (UI::MAX_SCALE + 1);
    let d_scale = d_scale % (UI::MAX_SCALE + 1);

    let n = Decimal::<UI, true>::try_from_parts(n_man, n_scale)?;
    let d = Decimal::<UI, true>::try_from_parts(d_man, d_scale)?;

    let n2 = OobScaleFpdec::<I>::from_mantissa(n_man);
    let d2 = OobScaleFpdec::<I>::from_mantissa(d_man);

    // mul
    match n.checked_mul(d) {
        Some(p) => {
            let (p_man, p_scale) = p.parts();

            let diff_scale = (n_scale + d_scale) as i32 - p_scale as i32;
            let p2 = n2.checked_mul(d2, diff_scale).unwrap();
            assert_eq!(p2.mantissa(), p_man);
        }
        None => {
            let diff_scale = (n_scale + d_scale) as i32;
            if let Some(p2) = n2.checked_mul(d2, diff_scale) {
                assert!(Decimal::<UI, true>::try_from_parts(p2.mantissa(), 0).is_none());
            }
        }
    }

    // div
    match n.checked_div(d) {
        Some(q) => {
            let (q_man, q_scale) = q.parts();

            let diff_scale = n_scale as i32 - d_scale as i32 - q_scale as i32;
            let q2 = n2.checked_div(d2, diff_scale).unwrap();

            let q2_man = q2.mantissa();

            // the 2 crate have different rounding choice if:
            //    n < 0 && d < 0 && r*2 == q
            if q2_man != q_man {
                assert!(n.is_negative());
                assert!(d.is_negative());
                assert_eq!(q_man - q2_man, I::ONE);
            }
        }
        None => {
            let diff_scale = n_scale as i32 - d_scale as i32;
            if let Some(q2) = n2.checked_div(d2, diff_scale) {
                assert!(Decimal::<UI, true>::try_from_parts(q2.mantissa(), 0).is_none());
            }
        }
    }

    Some(())
}

fn test_unsigned<I>(n_man: I, n_scale: u32, d_man: I, d_scale: u32) -> Option<()>
where
    I: FpdecInner + UnderlyingInt + std::fmt::Debug,
{
    let n_scale = n_scale % (I::MAX_SCALE + 1);
    let d_scale = d_scale % (I::MAX_SCALE + 1);

    let n = Decimal::<I, false>::try_from_parts(n_man, n_scale)?;
    let d = Decimal::<I, false>::try_from_parts(d_man, d_scale)?;

    let n2 = OobScaleFpdec::<I>::from_mantissa(n_man);
    let d2 = OobScaleFpdec::<I>::from_mantissa(d_man);

    // mul
    match n.checked_mul(d) {
        Some(p) => {
            let (p_man, p_scale) = p.parts();

            let diff_scale = (n_scale + d_scale) as i32 - p_scale as i32;
            // dbg!(n, d, p, n2, d2);
            let p2 = n2.checked_mul(d2, diff_scale).unwrap();
            assert_eq!(p2.mantissa(), p_man);
        }
        None => {
            let diff_scale = (n_scale + d_scale) as i32;
            if let Some(p2) = n2.checked_mul(d2, diff_scale) {
                assert!(Decimal::<I, false>::try_from_parts(p2.mantissa(), 0).is_none());
            }
        }
    }

    // div
    match n.checked_div(d) {
        Some(q) => {
            let (q_man, q_scale) = q.parts();

            let diff_scale = n_scale as i32 - d_scale as i32 - q_scale as i32;
            let q2 = n2.checked_div(d2, diff_scale).unwrap();

            let q2_man = q2.mantissa();
            assert_eq!(q2_man, q_man);
        }
        None => {
            let diff_scale = n_scale as i32 - d_scale as i32;
            if let Some(q2) = n2.checked_div(d2, diff_scale) {
                assert!(Decimal::<I, false>::try_from_parts(q2.mantissa(), 0).is_none());
            }
        }
    }

    Some(())
}
