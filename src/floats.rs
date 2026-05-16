use crate::{Decimal, UnderlyingInt};

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

/// Convert from `f32`, keeping about 7 significant digits.
///
/// # Examples:
///
/// ```
/// use decimax::Dec128;
/// let a = Dec128::try_from(1.234567_f32).unwrap();
/// let b = Dec128::from_parts(1234567, 6);
/// assert_eq!(a, b);
/// ```
impl<I: UnderlyingInt, const S: bool> TryFrom<f32> for Decimal<I, S> {
    type Error = FromFloatError;
    fn try_from(f: f32) -> Result<Self, Self::Error> {
        let (sign, man, exp) = decompose_f32(f)?;

        if !S && sign == 1 {
            return Err(FromFloatError::Negative);
        }

        let (man10, scale) = if exp < 0 {
            // exp < 0: man10 = (man2 * 10^scale) >> -exp
            let scale = -exp * 1233 >> 12;
            if scale as u32 > I::MAX_SCALE {
                return Err(FromFloatError::Underflow);
            }

            // calculate: man_scale_1 = (man * 10^scale) >> (-exp - 1)
            //                        = (man * 5^scale) >> (-exp - scale - 1)
            //
            // Tha @man takes 24 bits.
            // The max of @pow5, ALL_5EXPS[u128::MAX_SCALE], takes 72 bits.
            // So the multiplication will not overflow.
            let pow5 = ALL_5EXPS[scale as usize];
            let man_scaled_1 = ((man as u128 * pow5) >> (-exp - scale - 1)) as u32;

            let man_scaled = man_scaled_1 >> 1;
            let round = if man_scaled_1 & 1 == 1 { 1 } else { 0 };

            (I::from_u32(man_scaled + round), scale as u32)
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

/// Convert from `f64`, keeping about 15 significant digits.
///
/// This only works for 64-bit and 128-bit decimal types, but not for
/// 32-bit type, because `f64` has 53 bits mantissa.
///
/// # Examples:
///
/// ```
/// use decimax::Dec128;
/// let a = Dec128::try_from(1234567890.12345_f64).unwrap();
/// let b = Dec128::from_parts(123456789012345_i128, 5);
/// assert_eq!(a, b);
/// ```
impl<I: BigUnderlyingInt, const S: bool> TryFrom<f64> for Decimal<I, S> {
    type Error = FromFloatError;
    fn try_from(f: f64) -> Result<Self, Self::Error> {
        let (sign, man, exp) = decompose_f64(f)?;

        if !S && sign == 1 {
            return Err(FromFloatError::Negative);
        }

        let (man10, scale) = if exp < 0 {
            // exp < 0: man10 = (man2 * 10^scale) >> -exp
            let scale = -exp * 1233 >> 12;
            if scale as u32 > I::MAX_SCALE {
                return Err(FromFloatError::Underflow);
            }

            // calculate: man_scale_1 = (man * 10^scale) >> (-exp - 1)
            //                        = (man * 5^scale) >> (-exp - scale - 1)
            //
            // Tha @man takes 53 bits.
            // The max of @pow5, ALL_5EXPS[u128::MAX_SCALE], takes 72 bits.
            // 53 + 72 = 125. So the multiplication will not overflow. What a luck!
            let pow5 = ALL_5EXPS[scale as usize];
            let man_scaled_1 = ((man as u128 * pow5) >> (-exp - scale - 1)) as u64;

            let man_scaled = man_scaled_1 >> 1;
            let round = if man_scaled_1 & 1 == 1 { 1 } else { 0 };

            (I::from_u64(man_scaled + round), scale as u32)
        } else if exp == 0 {
            // integer, no scale
            (I::from_u64(man), 0)
        } else {
            // exp > 0: man10 = man2 << exp
            let exp = exp as u32;

            if exp + 24 > I::BITS - I::SCALE_BITS - S as u32 {
                return Err(FromFloatError::Overflow);
            }
            (I::from_u64(man) << exp, 0)
        };

        Ok(Decimal::pack(sign, scale, man10))
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
