mod int128;

use core::ops::{Add, AddAssign, BitAnd, BitOr, Div, Mul, Rem, Shl, Shr, Sub, SubAssign};

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct Decimal<I: UnderlyingInt>(I);

// Underlying Integer
pub trait UnderlyingInt:
    Sized
    + Clone
    + Copy
    + core::fmt::Debug
    + core::fmt::Display
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + Add<Output = Self>
    + Sub<Output = Self>
    + Div<Output = Self>
    + Rem<Output = Self>
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
    const MIN_UNDERINT: Self;

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

    // calculate: self * 2^extra_scale / right
    // return: quotient and remainder
    // The quotient may be bigger that MAX_MATISSA.
    fn div_with_extra_scale(self, right: Self, extra_scale: u32) -> Option<(Self, Self)>;

    fn div_with_avail_scale(self, right: Self, avail_scale: u32) -> (Self, Self, u32);

    fn div_rem(self, right: Self) -> (Self, Self);

    fn div_rem_exp(self, iexp: u32) -> (Self, Self);

    fn avail_digits(self) -> u32 {
        bits_to_digits(self.leading_zeros() - 1 - Self::SCALE_BITS)
    }
}

impl<I: UnderlyingInt> Decimal<I> {
    pub const ZERO: Self = Self(I::ZERO);
    pub const MAX: Self = Self(I::MAX_MATISSA);
    pub const MIN: Self = Self(I::MIN_UNDERINT);

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

    pub fn checked_add_exact(self, _right: Self) -> Option<Self> {
        todo!()
    }
    pub fn checked_mul_int(self, _right: Self) -> Option<Self> {
        todo!()
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

    pub fn checked_div(self, right: Self) -> Option<Self> {
        let (a_sign, a_scale, a_man) = self.unpack();
        let (b_sign, b_scale, b_man) = right.unpack();
        if b_man == I::ZERO {
            return None;
        }

        let ((mut q1, r1), mut diff_scale) = if a_scale >= b_scale {
            (a_man.div_rem(b_man), a_scale - b_scale)
        } else {
            (a_man.div_with_extra_scale(b_man, b_scale - a_scale)?, 0)
        };

        if r1 != I::ZERO {
            let avail_scale = q1.avail_digits().min(I::MAX_SCALE - diff_scale);
            let (q2, _r2, act_scale) = r1.div_with_avail_scale(b_man, avail_scale);
            q1 = q1.mul_exp(act_scale) + q2;
            // r1 = r2;
            diff_scale += act_scale;
        };

        Some(Self::do_pack(a_sign ^ b_sign, diff_scale, q1))
    }
}

impl<I: UnderlyingInt> Decimal<I> {
    pub fn is_zero(self) -> bool {
        let (_, _, man) = self.unpack();
        man == I::ZERO
    }

    pub fn is_positive(self) -> bool {
        let (sign, _, man) = self.unpack();
        sign == 0 && man != I::ZERO
    }

    pub fn is_negative(self) -> bool {
        let (sign, _, man) = self.unpack();
        sign != 0 && man != I::ZERO
    }
}

impl<I: UnderlyingInt> core::fmt::Debug for Decimal<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (sign, scale, man) = self.unpack();
        write!(f, "Decimal:[{sign} {scale} {man}]")
    }
}

// Mark this function as #[inline(never)] to avoid interfering with the
// main flow of the caller add_or_sub(), thereby enabling better compiler
// optimizations.
#[inline(never)]
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

    let small_avail = small_man.avail_digits();
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

pub(crate) fn bits_to_digits(bits: u32) -> u32 {
    bits * 1233 >> 12 // math!
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_div() {
        let a = Decimal::pack(0, 10, 1000).unwrap();
        let b = Decimal::pack(0, 10, 200).unwrap();
        let c = Decimal::pack(0, 10, 300).unwrap();
        assert_eq!(a.checked_div(b), Decimal::pack(0, 0, 5));
        assert_eq!(b.checked_div(a), Decimal::pack(0, 1, 2));
        assert_eq!(
            a.checked_div(c),
            Decimal::pack(0, 35, 3333333333_3333333333_3333333333_333333)
        );
    }
}
