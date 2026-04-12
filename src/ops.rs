use core::cmp::Ordering;
use core::fmt::{self, Debug};
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::{Decimal, UnderlyingInt};

impl<I: UnderlyingInt> Decimal<I> {
    pub fn abs(self) -> Self {
        Self(self.0 << 1 >> 1)
    }

    pub fn checked_add(self, right: Self) -> Option<Self> {
        let (b_sign, b_scale, b_man) = right.unpack();
        self.do_add(b_sign, b_scale, b_man)
    }

    pub fn checked_sub(self, right: Self) -> Option<Self> {
        let (b_sign, b_scale, b_man) = right.unpack();
        self.do_add(b_sign ^ 1, b_scale, b_man)
    }

    #[inline]
    fn do_add(self, b_sign: u8, b_scale: u32, b_man: I) -> Option<Self> {
        let (a_sign, a_scale, a_man) = self.unpack();

        let (a_man, b_man, scale) = if a_scale == b_scale {
            (a_man, b_man, a_scale)
        } else {
            align_scale(a_man, a_scale, b_man, b_scale)
        };

        // do the addition
        let (sign, sum) = if a_sign == b_sign {
            (a_sign, a_man + b_man)
        } else if a_man > b_man {
            (a_sign, a_man - b_man)
        } else {
            (b_sign, b_man - a_man)
        };

        // pack
        if sum <= I::MAX_MATISSA {
            Some(Self::pack(sign, scale, sum))
        } else if scale > 0 {
            let man = (sum + (I::TEN >> 1)) / I::TEN; // rounding-divide
            Some(Self::pack(sign, scale - 1, man))
        } else {
            None
        }
    }

    pub fn checked_mul(self, right: Self) -> Option<Self> {
        let (a_sign, a_scale, a_man) = self.unpack();
        let (b_sign, b_scale, b_man) = right.unpack();

        let (p_man, p_scale) = a_man.mul_with_sum_scale(b_man, a_scale + b_scale)?;

        Some(Self::pack(a_sign ^ b_sign, p_scale, p_man))
    }

    pub fn checked_mul_int<S>(self, i: S) -> Option<Self>
    where
        S: Into<I::Signed>,
    {
        let (a_sign, a_scale, a_man) = self.unpack();
        let (b_man, b_sign) = I::from_signed(i.into());

        let (p_man, p_scale) = a_man.mul_with_sum_scale(b_man, a_scale)?;

        Some(Self::pack(a_sign ^ b_sign, p_scale, p_man))
    }

    pub fn checked_div(self, right: Self) -> Option<Self> {
        let (a_sign, a_scale, a_man) = self.unpack();
        let (b_sign, b_scale, b_man) = right.unpack();
        if b_man == I::ZERO {
            return None;
        }

        let (q_man, q_scale) = a_man.div_with_scales(b_man, a_scale, b_scale)?;

        Some(Self::pack(a_sign ^ b_sign, q_scale, q_man))
    }

    pub fn checked_div_int<S>(self, i: S) -> Option<Self>
    where
        S: Into<I::Signed>,
    {
        let (a_sign, a_scale, a_man) = self.unpack();
        let (b_man, b_sign) = I::from_signed(i.into());
        if b_man == I::ZERO {
            return None;
        }

        let (q_man, q_scale) = a_man.div_with_scales(b_man, a_scale, 0)?;

        Some(Self::pack(a_sign ^ b_sign, q_scale, q_man))
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

impl<I: UnderlyingInt> Eq for Decimal<I> {}

impl<I: UnderlyingInt> PartialEq for Decimal<I> {
    fn eq(&self, other: &Self) -> bool {
        let (a_sign, a_scale, a_man) = self.unpack();
        let (b_sign, b_scale, b_man) = other.unpack();

        if a_man == I::ZERO {
            return b_man == I::ZERO;
        }
        if b_man == I::ZERO {
            return a_man == I::ZERO;
        }
        if a_sign != b_sign {
            return false;
        }
        if a_scale == b_scale {
            return a_man == b_man;
        }
        if a_scale < b_scale {
            match a_man.checked_mul_exp(b_scale - a_scale) {
                Some(a_man) => a_man == b_man,
                None => false,
            }
        } else {
            match b_man.checked_mul_exp(a_scale - b_scale) {
                Some(b_man) => b_man == a_man,
                None => false,
            }
        }
    }
}

impl<I: UnderlyingInt> Ord for Decimal<I> {
    fn cmp(&self, other: &Self) -> Ordering {
        let (a_sign, a_scale, a_man) = self.unpack();
        let (b_sign, b_scale, b_man) = other.unpack();

        if a_sign != b_sign {
            if a_man == I::ZERO && b_man == I::ZERO {
                Ordering::Equal
            } else if a_sign == 0 {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        } else {
            let ret = if a_scale == b_scale {
                a_man.cmp(&b_man)
            } else if a_scale < b_scale {
                match a_man.checked_mul_exp(b_scale - a_scale) {
                    Some(a_man) => a_man.cmp(&b_man),
                    None => Ordering::Greater,
                }
            } else {
                match b_man.checked_mul_exp(a_scale - b_scale) {
                    Some(b_man) => a_man.cmp(&b_man),
                    None => Ordering::Less,
                }
            };

            if a_sign == 0 {
                ret
            } else {
                match ret {
                    Ordering::Less => Ordering::Greater,
                    Ordering::Greater => Ordering::Less,
                    Ordering::Equal => Ordering::Equal,
                }
            }
        }
    }
}

impl<I: UnderlyingInt> PartialOrd for Decimal<I> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: UnderlyingInt> Neg for Decimal<I> {
    type Output = Self;
    fn neg(self) -> Self::Output {
        let sign = I::ONE << (I::BITS - 1);
        Self(self.0 ^ sign)
    }
}

impl<I: UnderlyingInt> Add for Decimal<I> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).expect("addition overflow")
    }
}

impl<I: UnderlyingInt> AddAssign for Decimal<I> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<I: UnderlyingInt> Sub for Decimal<I> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).expect("substraction overflow")
    }
}

impl<I: UnderlyingInt> SubAssign for Decimal<I> {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<I: UnderlyingInt> Mul for Decimal<I> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(rhs).expect("multiplication overflow")
    }
}

impl<I: UnderlyingInt> MulAssign for Decimal<I> {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<I: UnderlyingInt> Div for Decimal<I> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(rhs).expect("division overflow or by zero")
    }
}

impl<I: UnderlyingInt> DivAssign for Decimal<I> {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl<I: UnderlyingInt> core::iter::Sum for Decimal<I> {
    fn sum<Iter: Iterator<Item = Self>>(iter: Iter) -> Self {
        iter.fold(Self::ZERO, |acc, d| acc + d)
    }
}

impl<'a, I: UnderlyingInt> core::iter::Sum<&'a Self> for Decimal<I> {
    fn sum<Iter: Iterator<Item = &'a Self>>(iter: Iter) -> Self {
        iter.fold(Self::ZERO, |acc, d| acc + *d)
    }
}

impl<I: UnderlyingInt> Debug for Decimal<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (iman, scale) = self.parts();
        write!(f, "Decimal[{iman} {scale}]")
    }
}

// Mark this function as #[inline(never)] to avoid interfering with the
// main flow of the caller add_or_sub(), thereby enabling better compiler
// optimizations.
#[inline(never)]
fn align_scale<I>(mut a_man: I, a_scale: u32, mut b_man: I, b_scale: u32) -> (I, I, u32)
where
    I: UnderlyingInt,
{
    // big_man/big_scale: the number with bigger scale
    // small_man/small_scale: the number with smaller scale
    let (big_man, mut big_scale, small_man, small_scale) = if a_scale > b_scale {
        (&mut a_man, a_scale, &mut b_man, b_scale)
    } else {
        (&mut b_man, b_scale, &mut a_man, a_scale)
    };

    let small_avail = bits_to_digits(small_man.leading_zeros() - I::META_BITS);
    let diff = big_scale - small_scale;

    if diff <= small_avail {
        // rescale small_man to big_scale
        *small_man = small_man.mul_exp(diff);
    } else {
        // rescale both small_man and big_man
        *small_man = small_man.mul_exp(small_avail);
        *big_man = big_man.div_exp(diff - small_avail);
        big_scale = small_scale + small_avail;
    }

    (a_man, b_man, big_scale)
}

pub(crate) fn bits_to_digits(bits: u32) -> u32 {
    bits * 1233 >> 12 // math!
}
