mod int128;

use core::ops::{Add, AddAssign, BitOr, Div, Shl, Shr, Sub, SubAssign};

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
    + Shl<u8, Output = Self>
    + Shr<u8, Output = Self>
    + BitOr<Output = Self>
    + AddAssign
    + SubAssign
{
    const MAX: Self;
    const MIN: Self;
    const ZERO: Self;
    const TEN: Self;
    const BITS: u8;
    const MATISSA_DIGITS: u8;
    const SCALE_BITS: u8;
    const MAX_SCALE: u8;
    const IS_SIGNED: bool;

    fn as_u8(self) -> u8;
    fn from_u8(n: u8) -> Self;

    fn leading_zeros(self) -> u32;

    fn avail_digits(self) -> u8;

    // caller has made sure that no overflow
    fn mul_exp(self, iexp: u8) -> Self;

    fn div_exp(self, iexp: u8) -> Self;

    fn mul_with_sum_scale(self, right: Self, sum_scale: u8) -> Option<(Self, u8)>;
}

impl<I: UnderlyingInt> Decimal<I> {
    pub const ZERO: Self = Self(I::ZERO);

    pub fn mantissa(self) -> I {
        self.0 >> I::SCALE_BITS
    }

    pub fn scale(self) -> u8 {
        self.0.as_u8() & ((1 << I::SCALE_BITS) - 1)
    }

    pub fn unpack(self) -> (I, u8) {
        (self.mantissa(), self.scale())
    }

    pub fn pack(mantissa: I, scale: u8) -> Option<Self> {
        if !Self::valid_matissa(mantissa) {
            return None;
        }
        if scale > I::MAX_SCALE {
            return None;
        }
        Some(Self::do_pack(mantissa, scale))
    }

    // the caller must make sure that @mantissa and @scale are valid
    fn do_pack(mantissa: I, scale: u8) -> Self {
        debug_assert!(Self::valid_matissa(mantissa));
        debug_assert!(scale <= I::MAX_SCALE);
        Self((mantissa << I::SCALE_BITS) | I::from_u8(scale))
    }

    fn valid_matissa(m: I) -> bool {
        m <= I::MAX >> I::SCALE_BITS && m >= I::MIN >> I::SCALE_BITS
    }

    // the caller must make sure that @ma and @mb are valid
    fn pack_sum(sum: I, scale: u8) -> Option<Self> {
        if Self::valid_matissa(sum) {
            Some(Self::do_pack(sum, scale))
        } else if scale > 0 {
            Some(Self::do_pack(sum / I::TEN, scale - 1)) // TODO round div
        } else {
            None
        }
    }

    pub fn checked_add(self, right: Self) -> Option<Self> {
        let (a, b, scale) = self.align_scale(right);
        Self::pack_sum(a + b, scale)
    }

    pub fn checked_sub(self, right: Self) -> Option<Self> {
        let (a, b, scale) = self.align_scale(right);
        if !I::IS_SIGNED && a < b {
            return None;
        }
        Self::pack_sum(a - b, scale)
    }

    fn align_scale(self, right: Self) -> (I, I, u8) {
        let (a_man, a_scale) = self.unpack();
        let (b_man, b_scale) = right.unpack();

        // aligned already
        if a_scale == b_scale {
            return (a_man, b_man, a_scale);
        }

        // big_man/big_scale: the number with bigger scale
        // small_man/small_scale: the number with smaller scale
        let (big_man, big_scale, small_man, small_scale) = if a_scale > b_scale {
            (a_man, a_scale, b_man, b_scale)
        } else {
            (b_man, b_scale, a_man, a_scale)
        };

        let small_avail = small_man.avail_digits();
        let diff = big_scale - small_scale;

        if diff <= small_avail {
            (small_man.mul_exp(diff), big_man, big_scale)
        } else {
            Self::add_by_dividing_small(big_man, big_scale, small_man, small_avail, diff)
        }
    }

    // move the unlikely case into a separate cold function
    #[cold]
    fn add_by_dividing_small(
        big_man: I,
        big_scale: u8,
        small_man: I,
        small_avail: u8,
        diff: u8,
    ) -> (I, I, u8) {
        let small_man = small_man.mul_exp(diff - small_avail);
        let big_man = big_man.div_exp(small_avail);
        (small_man, big_man, big_scale - small_avail)
    }

    pub fn checked_mul(self, right: Self) -> Option<Self> {
        let (a_man, a_scale) = self.unpack();
        let (b_man, b_scale) = right.unpack();

        if a_man == I::ZERO || b_man == I::ZERO {
            return Some(Self::ZERO);
        }

        a_man
            .mul_with_sum_scale(b_man, a_scale + b_scale)
            .map(|(man, scale)| Self::do_pack(man, scale))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avail_digits() {
        assert_eq!(0_i128.avail_digits(), 36);

        let max = 2_i128.pow(121) - 1;
        assert_eq!(max.avail_digits(), 0);

        assert_eq!((max / 10 + 1).avail_digits(), 0);
        assert_eq!((max / 10).avail_digits(), 1);
        assert_eq!((max / 10 - 1).avail_digits(), 1);

        assert_eq!((max / 1000 + 1).avail_digits(), 2);
        assert_eq!((max / 1000).avail_digits(), 3);
        assert_eq!((max / 1000 - 1).avail_digits(), 3);
    }

    #[test]
    fn simple_add() {
        let a = Decimal::pack(1000, 10).unwrap();
        let b = Decimal::pack(1000, 13).unwrap();
        let r = Decimal::pack(1001000, 13).unwrap();
        assert_eq!(a.checked_add(b).unwrap(), r);
        assert_eq!(b.checked_add(a).unwrap(), r);
    }
}
