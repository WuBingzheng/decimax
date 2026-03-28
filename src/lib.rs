mod int128;

use core::ops::{Add, AddAssign, BitAnd, BitOr, Div, Mul, Shl, Shr, Sub, SubAssign};

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct Decimal<I: UnderlyingInt>(I);

// Underlying Integer
pub trait UnderlyingInt:
    Sized
    + Clone
    + Copy
    + core::fmt::Debug
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + Add<Output = Self>
    + Sub<Output = Self>
    + Div<Output = Self>
    + Mul<Output = Self>
    + Shl<u32, Output = Self>
    + Shr<u32, Output = Self>
    + BitOr<Output = Self>
    + BitAnd<Output = Self>
    + AddAssign
    + SubAssign
{
    const ZERO: Self;
    const ONE: Self;
    const TEN: Self;
    const MAX: Self;
    const MIN: Self;

    const BITS: u32;
    const SCALE_BITS: u32 = (Self::MAX_SCALE * 2 - 1).ilog2();
    const MAX_SCALE: u32;
    const MAX_MATISSA: Self;

    fn as_u32(self) -> u32;
    fn from_u32(n: u32) -> Self;

    fn leading_zeros(self) -> u32;

    // caller has made sure that no overflow
    fn mul_exp(self, iexp: u32) -> Self;

    fn div_exp(self, iexp: u32) -> Self;

    fn mul_with_sum_scale(self, right: Self, sum_scale: u32) -> Option<(Self, u32)>;

    fn min_avail_digits(self) -> u32 {
        let bits = self.leading_zeros() - Self::SCALE_BITS - 1;
        bits * 1233 >> 12 // math!
    }
}

impl<I: UnderlyingInt> Decimal<I> {
    pub const ZERO: Self = Self(I::ZERO);

    // layout:
    //   +-+-----+-------------+
    //   |S|scale|  mantissa   |
    //   +-+-----+-------------+
    pub fn unpack(self) -> (u8, u32, I) {
        let mantissa = self.0 & I::MAX_MATISSA;
        let meta = (self.0 >> (I::BITS - 1 - I::SCALE_BITS)).as_u32();
        let scale = meta & ((1 << I::SCALE_BITS) - 1);
        let sign = meta >> I::SCALE_BITS;
        (sign as u8, scale, mantissa)
    }

    pub fn pack(sign: u8, scale: u32, mantissa: I) -> Option<Self> {
        if sign > 1 {
            return None;
        }
        if scale > I::MAX_SCALE {
            return None;
        }
        if mantissa > I::MAX_MATISSA {
            return None;
        }
        Some(Self::do_pack(sign, scale, mantissa))
    }

    // the caller must make sure that the inputs are valid
    fn do_pack(sign: u8, scale: u32, mantissa: I) -> Self {
        debug_assert!(sign <= 1);
        debug_assert!(scale <= I::MAX_SCALE);
        debug_assert!(mantissa <= I::MAX_MATISSA);

        let meta = ((sign as u32) << I::SCALE_BITS) | scale;
        Self(I::from_u32(meta) << (I::BITS - 1 - I::SCALE_BITS) | mantissa)
    }

    pub fn checked_add(self, right: Self) -> Option<Self> {
        self.add_or_sub(right, true)
    }

    pub fn checked_sub(self, right: Self) -> Option<Self> {
        self.add_or_sub(right, false)
    }

    fn add_or_sub(self, right: Self, is_add: bool) -> Option<Self> {
        let (a_sign, a_scale, a_man) = self.unpack();
        let (b_sign, b_scale, b_man) = right.unpack();

        let (a_man, b_man, scale) = if a_scale == b_scale {
            (a_man, b_man, a_scale)
        } else {
            align_scale(a_man, a_scale, b_man, b_scale)
        };

        // do the addition or substraction
        let (sign, sum) = if (a_sign == b_sign) == is_add {
            (a_sign, a_man + b_man)
        } else if a_man > b_man {
            (a_sign, a_man - b_man)
        } else {
            (b_sign, b_man - a_man)
        };

        // pack
        if sum <= I::MAX_MATISSA {
            Some(Self::do_pack(sign, scale, sum))
        } else if scale > 0 {
            let man = (sum + (I::TEN >> 1)) / I::TEN; // rounding-divide
            Some(Self::do_pack(sign, scale - 1, man))
        } else {
            None
        }
    }

    pub fn checked_mul(self, right: Self) -> Option<Self> {
        let (a_sign, a_scale, a_man) = self.unpack();
        let (b_sign, b_scale, b_man) = right.unpack();

        let is_mul_overflow =
            a_man.leading_zeros() + b_man.leading_zeros() < I::BITS + I::SCALE_BITS + 1;

        let (man, scale) = if !is_mul_overflow {
            let p = a_man * b_man;
            let sum_scale = a_scale + b_scale;
            if sum_scale <= I::MAX_SCALE {
                (p, sum_scale)
            } else {
                (p.div_exp(sum_scale - I::MAX_SCALE), I::MAX_SCALE)
            }
        } else {
            a_man.mul_with_sum_scale(b_man, a_scale + b_scale)?
        };

        Some(Self::do_pack(a_sign ^ b_sign, scale, man))
    }
}

// Mark this function as #[cold] to avoid interfering with the main flow of the
// caller add_or_sub(), thereby enabling better compiler optimizations.
#[cold]
fn align_scale<I>(a_man: I, a_scale: u32, b_man: I, b_scale: u32) -> (I, I, u32)
where
    I: UnderlyingInt,
{
    // big_man/big_scale: the number with bigger scale
    // small_man/small_scale: the number with smaller scale
    let (big_man, big_scale, small_man, small_scale) = if a_scale > b_scale {
        (a_man, a_scale, b_man, b_scale)
    } else {
        (b_man, b_scale, a_man, a_scale)
    };

    let small_avail = small_man.min_avail_digits();
    let diff = big_scale - small_scale;

    if diff <= small_avail {
        // rescale small_man to big_scale
        (small_man.mul_exp(diff), big_man, big_scale)
    } else {
        // rescale both small_man and big_man
        let small_man = small_man.mul_exp(diff - small_avail);
        let big_man = big_man.div_exp(small_avail);
        (small_man, big_man, big_scale - small_avail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avail_digits() {
        assert_eq!(0_u128.min_avail_digits(), 36);

        let max = 2_u128.pow(121) - 1;
        assert_eq!(max.min_avail_digits(), 0);

        assert_eq!((max / 10 + 1).min_avail_digits(), 0);
        assert_eq!((max / 10).min_avail_digits(), 0);
        assert_eq!((max / 10 - 1).min_avail_digits(), 0);

        assert_eq!((max / 1000 + 1).min_avail_digits(), 2);
        assert_eq!((max / 1000).min_avail_digits(), 2);
        assert_eq!((max / 1000 - 1).min_avail_digits(), 2);
    }

    #[test]
    fn simple_add() {
        let a = Decimal::pack(0, 10, 1000).unwrap();
        let b = Decimal::pack(0, 13, 1000).unwrap();
        let r = Decimal::pack(0, 13, 1001000).unwrap();
        assert_eq!(a.checked_add(b).unwrap(), r);
        assert_eq!(b.checked_add(a).unwrap(), r);
    }

    #[test]
    fn test_mul() {
        let a = Decimal::pack(0, 10, 1000).unwrap();
        assert_eq!(a.checked_mul(a), Decimal::pack(0, 20, 1000_000));

        let a = Decimal::pack(0, 20, 1000).unwrap();
        assert_eq!(a.checked_mul(a), Decimal::pack(0, 36, 100));

        let a = Decimal::pack(0, 10, 10_u128.pow(20)).unwrap();
        assert_eq!(a.checked_mul(a), Decimal::pack(0, 16, 10_u128.pow(36)));

        // overflow: (30 + 30) - 36 > 10 + 10
        let a = Decimal::pack(0, 10, 10_u128.pow(30)).unwrap();
        assert_eq!(a.checked_mul(a), None);

        // (30 + 30) - 36 > 20 + 20
        let a = Decimal::pack(0, 20, 10_u128.pow(30)).unwrap();
        assert_eq!(a.checked_mul(a), Decimal::pack(0, 16, 10_u128.pow(36)));

        // (20 + 20) - 36 < 30 + 30
        let a = Decimal::pack(0, 30, 10_u128.pow(20)).unwrap();
        assert_eq!(a.checked_mul(a), Decimal::pack(0, 36, 10_u128.pow(16)));
    }
}
