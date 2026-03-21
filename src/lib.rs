// use num_traits::{
//     AsPrimitive, Num,
//     identities::{ConstOne, ConstZero, Zero},
//     int::PrimInt,
//     ops::wrapping::WrappingAdd,
// };

use core::ops::{Add, AddAssign, BitOr, Div, Mul, Shl, Shr, Sub, SubAssign};

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Default)]
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
    const SCALE_BITS: u8;
    const MAX_SCALE: u8;
    fn as_u8(self) -> u8;
    fn from_u8(n: u8) -> Self;

    fn get_exp(i: u8) -> Option<Self>;
    unsafe fn unchecked_get_exp(i: u8) -> Self;

    fn leading_zeros(self) -> u8;
}

const ALL_EXPS: [i128; 39] = [
    1,
    10_i128.pow(1),
    10_i128.pow(2),
    10_i128.pow(3),
    10_i128.pow(4),
    10_i128.pow(5),
    10_i128.pow(6),
    10_i128.pow(7),
    10_i128.pow(8),
    10_i128.pow(9),
    10_i128.pow(10),
    10_i128.pow(11),
    10_i128.pow(12),
    10_i128.pow(13),
    10_i128.pow(14),
    10_i128.pow(15),
    10_i128.pow(16),
    10_i128.pow(17),
    10_i128.pow(18),
    10_i128.pow(19),
    10_i128.pow(20),
    10_i128.pow(21),
    10_i128.pow(22),
    10_i128.pow(23),
    10_i128.pow(24),
    10_i128.pow(25),
    10_i128.pow(26),
    10_i128.pow(27),
    10_i128.pow(28),
    10_i128.pow(29),
    10_i128.pow(30),
    10_i128.pow(31),
    10_i128.pow(32),
    10_i128.pow(33),
    10_i128.pow(34),
    10_i128.pow(35),
    10_i128.pow(36),
    10_i128.pow(37),
    10_i128.pow(38),
];

impl UnderlyingInt for i128 {
    const MAX: Self = Self::MAX;
    const MIN: Self = Self::MIN;
    const ZERO: Self = 0;
    const TEN: Self = 10;
    const SCALE_BITS: u8 = 6;
    const MAX_SCALE: u8 = 36;

    fn as_u8(self) -> u8 {
        self as u8
    }
    fn from_u8(n: u8) -> Self {
        n as Self
    }
    fn get_exp(i: u8) -> Option<i128> {
        ALL_EXPS.get(i as usize).copied()
    }

    unsafe fn unchecked_get_exp(i: u8) -> i128 {
        unsafe { *ALL_EXPS.get_unchecked(i as usize) }
    }
    fn leading_zeros(self) -> u8 {
        self.leading_zeros() as u8
    }
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

        // let avail = 36 - smalli.ilog10();
        let small_avail = small_man.leading_zeros() / 3;
        let diff = big_scale - small_scale;

        if diff <= small_avail {
            // likely
            let exp = unsafe { I::unchecked_get_exp(diff) };
            (small_man * exp, big_man, big_scale)
        } else {
            Self::add_by_dividing_small(
                big_man,
                big_scale,
                small_man,
                small_scale,
                small_avail,
                diff,
            )
        }
    }

    // move the unlikely case into a separate cold function
    #[cold]
    fn add_by_dividing_small(
        big_man: I,
        big_scale: u8,
        small_man: I,
        small_scale: u8,
        small_avail: u8,
        diff: u8,
    ) -> (I, I, u8) {
        let small_man = small_man * unsafe { I::unchecked_get_exp(diff - small_scale) };
        let big_man = big_man / unsafe { I::unchecked_get_exp(small_avail) };
        (small_man, big_man, big_scale - small_avail)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let a = Decimal::pack(1000, 10);
//         let b = Decimal::pack(1000, 13);
//         let r = Decimal::pack(1001000, 13);
//         assert_eq!(a.checked_add(b).unwrap(), r);
//         assert_eq!(b.checked_add(a).unwrap(), r);
//     }
// }
