#![no_std]
#![doc = include_str!("../README.md")]

mod convert;
mod display;
mod from_str;
mod ops;

mod int128;
mod int32;
mod int64;

use core::fmt::{Debug, Display};
use core::ops::{Add, AddAssign, BitAnd, BitOr, BitXor, Div, Mul, Rem, Shl, Shr, Sub, SubAssign};

pub use from_str::ParseError;

/// The 128-bit decimal type, with about 36 significant digits and at most 31 fraction digits.
pub type Dec128 = Decimal<u128, true>;

/// The 128-bit unsigned decimal type, with about 36 significant digits and at most 31 fraction digits.
///
/// Since cargo generates full documentation only for the first alias ([`Dec128`]),
/// here is almost empty. See the [`Decimal`] page for full documentation of this alias.
pub type UDec128 = Decimal<u128, false>;

/// The 64-bit decimal type, with about 18 significant digits and at most 15 fraction digits.
///
/// Since cargo generates full documentation only for the first alias ([`Dec128`]),
/// here is almost empty. See the [`Decimal`] page for full documentation of this alias.
pub type Dec64 = Decimal<u64, true>;

/// The 64-bit unsigned decimal type, with about 18 significant digits and at most 15 fraction digits.
///
/// Since cargo generates full documentation only for the first alias ([`Dec128`]),
/// here is almost empty. See the [`Decimal`] page for full documentation of this alias.
pub type UDec64 = Decimal<u64, false>;

/// The 32-bit decimal type, with about 9 significant digits and at most 7 fraction digits.
///
/// Since cargo generates full documentation only for the first alias ([`Dec128`]),
/// here is almost empty. See the [`Decimal`] page for full documentation of this alias.
pub type Dec32 = Decimal<u32, true>;

/// The 32-bit unsigned decimal type, with about 9 significant digits and at most 7 fraction digits.
///
/// Since cargo generates full documentation only for the first alias ([`Dec128`]),
/// here is almost empty. See the [`Decimal`] page for full documentation of this alias.
pub type UDec32 = Decimal<u32, false>;

/// The decimal type.
///
/// The `I` is the underlying integer type, which could be `u128`, `u64` or `u32`.
///
/// The `S` is for signed or unsigned.
///
/// They have aliases [`Dec128`], [`Dec64`], [`Dec32`], [`UDec128`], [`UDec64`]
/// and [`UDec32`] respectively.
#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct Decimal<I: UnderlyingInt, const S: bool>(I);

/// Underlying integer type.
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
    const MAX: Self;
    const ZERO: Self;
    const ONE: Self;
    const TEN: Self;
    const HUNDRED: Self;

    const UNSIGNED_MAX_MATISSA: Self;
    const SIGNED_MAX_MATISSA: Self;
    const SIGNED_MIN_UNDERINT: Self;

    const BITS: u32;
    const SCALE_BITS: u32;
    const MAX_SCALE: u32 = 2_u32.pow(Self::SCALE_BITS) - 1;

    type Signed: Display;

    // to pack and unpack
    fn to_signed(self, sign: u8) -> Self::Signed;
    fn from_signed(s: Self::Signed) -> (Self, u8);

    // to decode and encode the scale
    fn as_u32(self) -> u32;
    fn from_u32(n: u32) -> Self;

    fn leading_zeros(self) -> u32;

    // caller has made sure that no overflow
    fn mul_exp(self, iexp: u32) -> Self;

    // the implementation need to check if overflow
    fn checked_mul_exp(self, iexp: u32) -> Option<Self>;

    // caller has made sure that @iexp is in range
    // remember to round the result
    fn div_exp(self, iexp: u32) -> Self;

    // caller has made sure that @iexp is in range
    fn div_rem_exp(self, iexp: u32) -> (Self, Self);

    // calculate `self * right` with sum of scales
    fn mul_with_sum_scale<const S: bool>(self, right: Self, sum_scale: u32) -> Option<(Self, u32)>;

    // calculate `self / right` with scales
    fn div_with_scales<const S: bool>(
        self,
        d: Self,
        s_scale: u32,
        d_scale: u32,
    ) -> Option<(Self, u32)>;
}

// for signed and unsigned both
impl<I: UnderlyingInt, const S: bool> Decimal<I, S> {
    /// Zero.
    pub const ZERO: Self = Self(I::ZERO);

    /// One.
    pub const ONE: Self = Self(I::ONE);

    const META_BITS: u32 = (S as u32) + I::SCALE_BITS;
    const MANTISSA_BITS: u32 = I::BITS - Self::META_BITS;

    // layout:
    //   +-+-----+-------------+
    //   |S|scale|  mantissa   |
    //   +-+-----+-------------+
    fn unpack(self) -> (u8, u32, I) {
        if S {
            let mantissa = self.0 & I::SIGNED_MAX_MATISSA;
            let meta = (self.0 >> Self::MANTISSA_BITS).as_u32();
            let scale = meta & ((1 << I::SCALE_BITS) - 1);
            let sign = meta >> I::SCALE_BITS;
            (sign as u8, scale, mantissa)
        } else {
            let mantissa = self.0 & I::UNSIGNED_MAX_MATISSA;
            let scale = (self.0 >> Self::MANTISSA_BITS).as_u32();
            (0, scale, mantissa)
        }
    }

    // the caller must make sure that the inputs are valid
    fn pack(sign: u8, scale: u32, mantissa: I) -> Self {
        let meta = if S {
            debug_assert!(sign <= 1);
            debug_assert!(scale <= I::MAX_SCALE);
            debug_assert!(Self::valid_mantissa(mantissa));

            ((sign as u32) << I::SCALE_BITS) | scale
        } else {
            debug_assert!(sign == 0);
            debug_assert!(scale <= I::MAX_SCALE);
            debug_assert!(Self::valid_mantissa(mantissa));

            scale
        };

        Self(I::from_u32(meta) << Self::MANTISSA_BITS | mantissa)
    }

    fn valid_mantissa(man: I) -> bool {
        man <= (I::MAX >> Self::META_BITS)
    }

    /// Return the underlying integer.
    ///
    /// You should not try to decode this integer yourself.
    /// You should just hold this and load it later.
    pub const fn underlying(self) -> I {
        self.0
    }

    /// Load from underlying integer, which should only be got from [`Self::underlying`].
    pub const fn from_underlying(i: I) -> Self {
        Self(i)
    }
}

impl<I: UnderlyingInt> Decimal<I, true> {
    /// The largest value.
    pub const MAX: Self = Self(I::SIGNED_MAX_MATISSA);

    /// The smallest value. It's the negative of [`Self::MAX`].
    pub const MIN: Self = Self(I::SIGNED_MIN_UNDERINT);

    /// Deconstruct the decimal into signed mantissa and scale.
    ///
    /// # Examples:
    ///
    /// ```
    /// use lean_decimal::Dec128;
    /// use core::str::FromStr;
    ///
    /// let d = Dec128::from_str("3.14").unwrap();
    /// assert_eq!(d.parts(), (314, 2));
    /// ```
    pub fn parts(self) -> (I::Signed, u32) {
        let (sign, scale, man) = self.unpack();
        (I::to_signed(man, sign), scale)
    }

    /// Construct a decimal from signed mantissa and scale.
    ///
    /// # Panic:
    ///
    /// Panic if the mantissa or scale is out of range. Use [`Self::try_from_parts`]
    /// for the checked version.
    ///
    /// # Examples:
    ///
    /// ```
    /// use lean_decimal::Dec128;
    /// let d = Dec128::from_parts(314, 2);
    /// assert_eq!(d.to_string(), "3.14");
    /// ```
    pub fn from_parts<S>(mantissa: S, scale: u32) -> Self
    where
        S: Into<I::Signed>,
    {
        Self::try_from_parts(mantissa, scale).expect("invalid decimal input parts")
    }

    /// Construct a decimal from signed mantissa and scale.
    ///
    /// Return `None` if the mantissa or scale is out of range.
    ///
    /// # Examples:
    ///
    /// ```
    /// use lean_decimal::Dec128;
    /// let d = Dec128::try_from_parts(314, 2).unwrap();
    /// assert_eq!(d.to_string(), "3.14");
    ///
    /// let d = Dec128::try_from_parts(314, 99); // 99 is out of range
    /// assert!(d.is_none());
    /// ```
    pub fn try_from_parts<S>(mantissa: S, scale: u32) -> Option<Self>
    where
        S: Into<I::Signed>,
    {
        if scale > I::MAX_SCALE {
            return None;
        }
        let (man, sign) = I::from_signed(mantissa.into());
        if man > I::SIGNED_MAX_MATISSA {
            return None;
        }
        Some(Self::pack(sign, scale, man))
    }
}

impl<I: UnderlyingInt> Decimal<I, false> {
    /// The largest value of unsigned decimal.
    pub const MAX: Self = Self(I::UNSIGNED_MAX_MATISSA);

    /// The smallest value of unsigned decimal. It's zero.
    pub const MIN: Self = Self::ZERO;

    /// Deconstruct the unsigned decimal into mantissa and scale.
    ///
    /// # Examples:
    ///
    /// ```
    /// use lean_decimal::UDec128;
    /// use core::str::FromStr;
    ///
    /// let d = UDec128::from_str("3.14").unwrap();
    /// assert_eq!(d.parts(), (314, 2));
    /// ```
    pub fn parts(self) -> (I, u32) {
        let (_, scale, man) = self.unpack();
        (man, scale)
    }

    /// Construct a unsigned decimal from mantissa and scale.
    ///
    /// # Panic:
    ///
    /// Panic if the mantissa or scale is out of range. Use [`Self::try_from_parts`]
    /// for the checked version.
    ///
    /// # Examples:
    ///
    /// ```
    /// use lean_decimal::UDec128;
    /// let d = UDec128::from_parts(314, 2);
    /// assert_eq!(d.to_string(), "3.14");
    /// ```
    pub fn from_parts(mantissa: I, scale: u32) -> Self {
        Self::try_from_parts(mantissa, scale).expect("invalid decimal input parts")
    }

    /// Construct a unsigned decimal from signed mantissa and scale.
    ///
    /// Return `None` if the mantissa or scale is out of range.
    ///
    /// # Examples:
    ///
    /// ```
    /// use lean_decimal::UDec128;
    /// let d = UDec128::try_from_parts(314, 2).unwrap();
    /// assert_eq!(d.to_string(), "3.14");
    ///
    /// let d = UDec128::try_from_parts(314, 99); // 99 is out of range
    /// assert!(d.is_none());
    /// ```
    pub fn try_from_parts(mantissa: I, scale: u32) -> Option<Self> {
        if scale > I::MAX_SCALE {
            return None;
        }
        if mantissa > I::UNSIGNED_MAX_MATISSA {
            return None;
        }
        Some(Self::pack(0, scale, mantissa))
    }
}

pub(crate) fn bits_to_digits(bits: u32) -> u32 {
    bits * 1233 >> 12 // math!
}
