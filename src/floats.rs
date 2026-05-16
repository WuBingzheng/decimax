use crate::{Decimal, UnderlyingInt, bits_to_digits};

/// Only for `u64` and `u128`, but no `u32`.
///
/// This is used for converting from `f64`, which has 53 bits mantissa,
/// bigger than `u32`.
pub trait BigUnderlyingInt: UnderlyingInt {
    fn from_u64(v: u64) -> Self;
}

impl BigUnderlyingInt for u64 {
    fn from_u64(v: u64) -> Self {
        v
    }
}

impl BigUnderlyingInt for u128 {
    fn from_u64(v: u64) -> Self {
        v as u128
    }
}

/// Error in converting from string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FromFloatError {
    Subnormal,
    Infinity,
    Nan,
    Underflow,
    Overflow,
    /// The float is negative but the decimal is unsigned.
    Negative,
}

fn decompose_f32(f: f32) -> Result<(u8, u32, i32), FromFloatError> {
    let bits = f.to_bits();

    let sign = ((bits >> 31) & 1) as u8;
    let exp = ((bits >> 23) & 0xff) as i32;
    let man = bits & 0x7fffff;

    if exp == 0 {
        if man == 0 {
            Ok((0, 0, 0))
        } else {
            Err(FromFloatError::Subnormal)
        }
    } else if exp == 0xff {
        if man == 0 {
            Err(FromFloatError::Infinity)
        } else {
            Err(FromFloatError::Nan)
        }
    } else {
        // normal cases
        let man = (1 << 23) | man;
        let exp = exp - 127 - 23;
        Ok((sign, man, exp))
    }
}

fn decompose_f64(f: f64) -> Result<(u8, u64, i32), FromFloatError> {
    let bits = f.to_bits();

    let sign = ((bits >> 63) & 1) as u8;
    let exp = ((bits >> 52) & 0x7ff) as i32;
    let man = bits & 0xf_ffff_ffff_ffff;

    if exp == 0 {
        if man == 0 {
            Ok((0, 0, 0))
        } else {
            Err(FromFloatError::Subnormal)
        }
    } else if exp == 0x7ff {
        if man == 0 {
            Err(FromFloatError::Infinity)
        } else {
            Err(FromFloatError::Nan)
        }
    } else {
        // normal cases
        let man = (1 << 52) | man;
        let exp = exp - 1023 - 52;
        Ok((sign, man, exp))
    }
}

impl<I: UnderlyingInt, const S: bool> TryFrom<f32> for Decimal<I, S> {
    type Error = FromFloatError;

    /// Convert from `f32`, keeping about 7 significant digits.
    ///
    /// # Examples:
    ///
    /// ```
    /// let a = decimax::Dec128::try_from(1.234_f32).unwrap();
    /// assert_eq!(a.parts(), (1234000, 6));
    ///
    /// // It may loss precision for small numbers.
    /// // For example Dec32 has 7 fraction precision at most.
    /// let a = decimax::Dec32::try_from(5.123e-7_f32).unwrap();
    /// assert_eq!(a, decimax::Dec32::from_parts(5, 7));
    /// ```
    fn try_from(f: f32) -> Result<Self, Self::Error> {
        let (sign, man, exp) = decompose_f32(f)?;

        if !S && sign == 1 {
            return Err(FromFloatError::Negative);
        }

        let (man10, scale) = if exp < 0 {
            // exp < 0: man10 = (man2 * 10^scale) >> -exp
            let exp = (-exp) as u32;

            let scale = bits_to_digits(exp).min(I::MAX_SCALE);

            // calculate: man_scale = (man * 10^scale) >> exp
            //                      = (man * 5^scale) >> (exp - scale)
            //
            // Tha @man has 24 bits, the max of @ALL_5EXPS has 72 bits,
            // so the multiplication will not overflow.
            let man10 = round_mul_shl(man as u128, ALL_5EXPS[scale as usize], exp - scale);
            if man10 == 0 {
                return Err(FromFloatError::Underflow);
            }

            (I::from_u32(man10 as u32), scale)
        } else if exp == 0 {
            // integer, no scale
            (I::from_u32(man), 0)
        } else {
            // exp > 0: man10 = man2 << exp
            let exp = exp as u32;

            if exp + 24 > I::BITS - I::SCALE_BITS - S as u32 {
                return Err(FromFloatError::Overflow);
            }
            (I::from_u32(man) << exp, 0)
        };

        Ok(Decimal::pack(sign, scale, man10))
    }
}

impl<I: BigUnderlyingInt, const S: bool> TryFrom<f64> for Decimal<I, S> {
    type Error = FromFloatError;

    /// Convert from `f64`, keeping about 15 significant digits.
    ///
    /// If you do not need so many significant digits, use `f32` instead.
    ///
    /// This only works for 64-bit and 128-bit decimal types, but not for
    /// 32-bit type, because `f64` has 53 bits mantissa.
    ///
    /// # Examples:
    ///
    /// ```
    /// let a = decimax::Dec128::try_from(1.234_f64).unwrap();
    /// assert_eq!(a.parts(), (1234000000000000, 15)); // big mantissa
    /// ```
    fn try_from(f: f64) -> Result<Self, Self::Error> {
        let (sign, man, exp) = decompose_f64(f)?;

        if !S && sign == 1 {
            return Err(FromFloatError::Negative);
        }

        let (man10, scale) = if exp < 0 {
            // exp < 0: man10 = (man2 * 10^scale) >> -exp
            let exp = (-exp) as u32;
            let scale = bits_to_digits(exp).min(I::MAX_SCALE);

            // calculate: man_scale = (man * 10^scale) >> exp
            //                      = (man * 5^scale) >> (exp - scale)
            //
            // Tha @man has 53 bits, the max of @ALL_5EXPS has 72 bits,
            // so the multiplication will not overflow. What a luck!
            let man10 = round_mul_shl(man as u128, ALL_5EXPS[scale as usize], exp - scale);
            if man10 == 0 {
                return Err(FromFloatError::Underflow);
            }

            (I::from_u64(man10 as u64), scale)
        } else if exp == 0 {
            // integer, no scale
            (I::from_u64(man), 0)
        } else {
            // exp > 0: man10 = man2 << exp
            let exp = exp as u32;

            if exp + 53 > I::BITS - I::SCALE_BITS - S as u32 {
                return Err(FromFloatError::Overflow);
            }
            (I::from_u64(man) << exp, 0)
        };

        Ok(Decimal::pack(sign, scale, man10))
    }
}

// Calculate round: (n * m) >> sh
// The caller should make sure no overflow.
fn round_mul_shl(n: u128, m: u128, sh: u32) -> u128 {
    let p1 = (n * m) >> (sh - 1);
    let p0 = p1 >> 1;
    let round = if p1 & 1 == 1 { 1 } else { 0 };
    p0 + round
}

// Decimal -> floats

impl<I: UnderlyingInt, const S: bool> From<Decimal<I, S>> for f32 {
    /// Convert decimal to `f32`.
    ///
    /// # Examples:
    ///
    /// ```
    /// let f = 12.3456_f32;
    /// let a = decimax::Dec128::try_from(f).unwrap();
    /// assert_eq!(f32::from(a), f);
    /// ```
    fn from(value: Decimal<I, S>) -> Self {
        let (sign, scale, man) = value.unpack();
        let f = man.to_f32() / ALL_EXPS_F32[scale as usize];
        if sign == 0 { f } else { -f }
    }
}

impl<I: UnderlyingInt, const S: bool> From<Decimal<I, S>> for f64 {
    /// Convert decimal to `f64`.
    ///
    /// # Examples:
    ///
    /// ```
    /// let f = 12345678.12345678_f64;
    /// let a = decimax::Dec128::try_from(f).unwrap();
    /// assert_eq!(f64::from(a), f);
    /// ```
    fn from(value: Decimal<I, S>) -> Self {
        let (sign, scale, man) = value.unpack();
        let f = man.to_f64() / ALL_EXPS_F64[scale as usize];
        if sign == 0 { f } else { -f }
    }
}

// 31 = u128::MAX_SCALE
const ALL_5EXPS: [u128; 32] = [
    1,
    5_u128.pow(1),
    5_u128.pow(2),
    5_u128.pow(3),
    5_u128.pow(4),
    5_u128.pow(5),
    5_u128.pow(6),
    5_u128.pow(7),
    5_u128.pow(8),
    5_u128.pow(9),
    5_u128.pow(10),
    5_u128.pow(11),
    5_u128.pow(12),
    5_u128.pow(13),
    5_u128.pow(14),
    5_u128.pow(15),
    5_u128.pow(16),
    5_u128.pow(17),
    5_u128.pow(18),
    5_u128.pow(19),
    5_u128.pow(20),
    5_u128.pow(21),
    5_u128.pow(22),
    5_u128.pow(23),
    5_u128.pow(24),
    5_u128.pow(25),
    5_u128.pow(26),
    5_u128.pow(27),
    5_u128.pow(28),
    5_u128.pow(29),
    5_u128.pow(30),
    5_u128.pow(31),
];

// 31 = u128::MAX_SCALE
const ALL_EXPS_F32: [f32; 32] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22, 1e23, 1e24, 1e25, 1e26, 1e27, 1e28, 1e29, 1e30, 1e31,
];
// 31 = u128::MAX_SCALE
const ALL_EXPS_F64: [f64; 32] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22, 1e23, 1e24, 1e25, 1e26, 1e27, 1e28, 1e29, 1e30, 1e31,
];
