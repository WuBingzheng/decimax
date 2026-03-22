mod int128;

use core::ops::{Add, AddAssign, BitOr, Div, Mul, Shl, Shr, Sub, SubAssign};

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
    + Mul<Output = Self>
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
    fn as_u8(self) -> u8;
    fn from_u8(n: u8) -> Self;

    fn get_exp(i: u8) -> Self;

    fn leading_zeros(self) -> u32;

    fn avail_digits(self) -> u8;
    fn mul_div_exp(self, right: Self, iexp: u8) -> Self;
}

impl<I: UnderlyingInt> Decimal<I> {
    pub const ZERO: Self = Self(I::ZERO);

    pub fn mantissa(self) -> I {
        self.0 >> I::SCALE_BITS
    }

    fn scale(self) -> u8 {
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
            Some(Self::do_pack(sum / I::TEN, scale - 1))
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
        Self::pack_sum(a - b, scale) // TODO checked_sub 
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
            (small_man * I::get_exp(diff), big_man, big_scale)
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
        let small_man = small_man * I::get_exp(diff - small_avail);
        let big_man = big_man / I::get_exp(small_avail);
        (small_man, big_man, big_scale - small_avail)
    }

    pub fn checked_mul(self, right: Self) -> Option<Self> {
        let (a_man, a_scale) = self.unpack();
        let (b_man, b_scale) = right.unpack();

        let sum_scale = a_scale + b_scale;
        if sum_scale <= I::MAX_SCALE {
            // TODO sub signal bit?
            if ((a_man.leading_zeros() + b_man.leading_zeros()) as u8) < I::BITS - I::SCALE_BITS {
                Some(Self::do_pack(a_man * b_man, sum_scale))
            } else {
                let i = 3;
                let man = I::mul_div_exp(a_man, b_man, i);
                Some(Self::do_pack(man, sum_scale - i))
            }
        } else {
            let diff = I::MAX_SCALE - sum_scale;
            if ((a_man.leading_zeros() + b_man.leading_zeros()) as u8) < I::BITS - I::SCALE_BITS {
                Some(Self::do_pack(a_man * b_man / I::TEN, I::MAX_SCALE))
            } else {
                let man = I::mul_div_exp(a_man, b_man, diff);
                Some(Self::do_pack(man, I::MAX_SCALE))
            }
        }
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
