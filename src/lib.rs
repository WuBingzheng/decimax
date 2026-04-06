mod int128;

use core::cmp::Ordering;
use core::fmt::{self, Debug, Display};
use core::ops::{
    Add, AddAssign, BitAnd, BitOr, BitXor, Div, Mul, Neg, Rem, Shl, Shr, Sub, SubAssign,
};

#[derive(Copy, Clone, Hash, Default)]
#[repr(transparent)]
pub struct Decimal<I: UnderlyingInt>(I);

// Underlying Integer
pub trait UnderlyingInt:
    Sized
    + Clone
    + Copy
    + Debug
    + Display
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
    + BitXor<Output = Self>
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

    type Signed: Display;

    fn to_signed(self, sign: u8) -> Self::Signed;
    fn from_signed(s: Self::Signed) -> (Self, u8);

    fn as_u32(self) -> u32;
    fn from_u32(n: u32) -> Self;

    fn leading_zeros(self) -> u32;

    // caller has made sure that no overflow
    fn mul_exp(self, iexp: u32) -> Self;

    fn checked_mul_exp(self, iexp: u32) -> Option<Self>;

    fn div_exp(self, iexp: u32) -> Self;

    fn mul_with_sum_scale(self, right: Self, sum_scale: u32) -> Option<(Self, u32)>;

    fn div_with_scales(self, d: Self, s_scale: u32, d_scale: u32) -> Option<(Self, u32)>;

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
    fn unpack(self) -> (u8, u32, I) {
        let mantissa = self.0 & I::MAX_MATISSA;
        let meta = (self.0 >> (I::BITS - 1 - I::SCALE_BITS)).as_u32();
        let scale = meta & ((1 << I::SCALE_BITS) - 1);
        let sign = meta >> I::SCALE_BITS;
        (sign as u8, scale, mantissa)
    }

    // the caller must make sure that the inputs are valid
    fn pack(sign: u8, scale: u32, mantissa: I) -> Self {
        debug_assert!(sign <= 1);
        debug_assert!(scale <= I::MAX_SCALE);
        debug_assert!(mantissa <= I::MAX_MATISSA);

        let meta = ((sign as u32) << I::SCALE_BITS) | scale;
        Self(I::from_u32(meta) << (I::BITS - 1 - I::SCALE_BITS) | mantissa)
    }

    pub fn parts(self) -> (I::Signed, u32) {
        let (sign, scale, man) = self.unpack();
        (I::to_signed(man, sign), scale)
    }

    pub fn from_parts(mantissa: I::Signed, scale: u32) -> Self {
        Self::try_from_parts(mantissa, scale).unwrap()
    }

    pub fn try_from_parts(mantissa: I::Signed, scale: u32) -> Option<Self> {
        if scale > I::MAX_SCALE {
            return None;
        }
        let (man, sign) = I::from_signed(mantissa);
        if man > I::MAX_MATISSA {
            return None;
        }
        Some(Self::pack(sign, scale, man))
    }
}

impl<I: UnderlyingInt> Decimal<I> {
    pub fn checked_add_exact(self, _right: Self) -> Option<Self> {
        todo!()
    }
    pub fn checked_mul_int(self, _right: Self) -> Option<Self> {
        todo!()
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

        Some(Self::pack(a_sign ^ b_sign, scale, man))
    }

    pub fn checked_div(self, right: Self) -> Option<Self> {
        let (a_sign, a_scale, a_man) = self.unpack();
        let (b_sign, b_scale, b_man) = right.unpack();
        if b_man == I::ZERO {
            return None;
        }

        let (q, scale) = a_man.div_with_scales(b_man, a_scale, b_scale)?;

        Some(Self::pack(a_sign ^ b_sign, scale, q))
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
        self.partial_cmp(other).unwrap()
    }
}

impl<I: UnderlyingInt> PartialOrd for Decimal<I> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
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
        .into() // Some()
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
        self.checked_add(rhs).unwrap()
    }
}

impl<I: UnderlyingInt> Sub for Decimal<I> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).unwrap()
    }
}

impl<I: UnderlyingInt> Mul for Decimal<I> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(rhs).unwrap()
    }
}

impl<I: UnderlyingInt> Div for Decimal<I> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(rhs).unwrap()
    }
}

impl<I: UnderlyingInt> Debug for Decimal<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (iman, scale) = self.parts();
        write!(f, "Decimal:[{iman} {scale}]")
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

    let small_avail = small_man.avail_digits();
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
