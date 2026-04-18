#![no_std]
#![doc = include_str!("../README.md")]

mod display;
mod from_str;
mod ops;

mod int128;
// mod int64;
// mod int32;

use core::fmt::{Debug, Display};
use core::ops::{Add, AddAssign, BitAnd, BitOr, BitXor, Div, Mul, Rem, Shl, Shr, Sub, SubAssign};

pub use from_str::ParseError;

/// The 128-bit decimal type, with about 36 significant digits in base-10 and scale in [0, 36].
pub type Dec128 = Decimal<u128>;

/// The decimal type.
///
/// The `I` is the underlying integer type, which could be `u32`, `u64`, or `u128`.
/// They have aliases [`Dec32`], [`Dec64`] and [`Dec128`] respectively.
#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct Decimal<I: UnderlyingInt>(I);

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
    const ZERO: Self;
    const ONE: Self;
    const TEN: Self;
    const HUNDRED: Self;
    const MAX_MATISSA: Self;
    const MIN_UNDERINT: Self;

    const BITS: u32;
    const MAX_SCALE: u32;
    const SCALE_BITS: u32 = (Self::MAX_SCALE * 2 - 1).ilog2();
    const META_BITS: u32 = 1 + Self::SCALE_BITS;
    const MATISSA_BITS: u32 = Self::BITS - Self::META_BITS;

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
    fn mul_with_sum_scale(self, right: Self, sum_scale: u32) -> Option<(Self, u32)>;

    // calculate `self / right` with scales
    fn div_with_scales(self, d: Self, s_scale: u32, d_scale: u32) -> Option<(Self, u32)>;
}

impl<I: UnderlyingInt> Decimal<I> {
    /// Zero.
    pub const ZERO: Self = Self(I::ZERO);

    /// The largest value. To be largest, the scale is 0, so this is an
    /// integer, `2<sup>b</sup> - 1`, where `b` is the mantissa bits,
    /// which is x, x, 36 for `u32`, `u64`, `u128` correspondingly.
    pub const MAX: Self = Self(I::MAX_MATISSA);

    /// The smallest value. It's the negative of [`Self::MAX`].
    pub const MIN: Self = Self(I::MIN_UNDERINT);

    // layout:
    //   +-+-----+-------------+
    //   |S|scale|  mantissa   |
    //   +-+-----+-------------+
    fn unpack(self) -> (u8, u32, I) {
        let mantissa = self.0 & I::MAX_MATISSA;
        let meta = (self.0 >> I::MATISSA_BITS).as_u32();
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
        Self(I::from_u32(meta) << I::MATISSA_BITS | mantissa)
    }

    /// Deconstruct the decimal into signed mantissa and scale.
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
        if man > I::MAX_MATISSA {
            return None;
        }
        Some(Self::pack(sign, scale, man))
    }

    /// Return the underlying integer.
    ///
    /// You should not try to decode this integer yourself.
    /// You should just hold this and load it later.
    pub const fn underlying(self) -> I {
        self.0
    }

    /// Load from underlying integer, which should only be got from [`Self::underlying`].
    ///
    /// # Panic:
    ///
    /// Panic if invalid. Use [`Self::try_from_underlying`] for checked version.
    pub fn from_underlying(ui: I) -> Self {
        Self::try_from_underlying(ui).expect("invalid underlying integer")
    }

    /// Load from underlying integer, which should only be got from [`Self::underlying`].
    pub fn try_from_underlying(ui: I) -> Option<Self> {
        let meta = (ui >> I::MATISSA_BITS).as_u32();
        let scale = meta & ((1 << I::SCALE_BITS) - 1);
        if scale > I::MAX_SCALE {
            None
        } else {
            Some(Self(ui))
        }
    }

    /// Load from underlying integer, which should only be got from [`Self::underlying`].
    ///
    /// SAFETY: It's safe if you make sure the integer was got from [`Self::underlying`].
    pub const unsafe fn unchecked_from_underlying(ui: I) -> Self {
        Self(ui)
    }
}

macro_rules! convert_from_int {
    ($decimal_int:ty, $decimal_signed:ty; $($from_int:ty),*) => {$(
        impl From<$from_int> for Decimal<$decimal_int> {
            fn from(value: $from_int) -> Self {
                Self::from_parts(value as $decimal_signed, 0)
            }
        }
    )*};
}
convert_from_int!(u128, i128; i8, u8, i16, u16, i32, u32, i64, u64);

pub(crate) fn bits_to_digits(bits: u32) -> u32 {
    bits * 1233 >> 12 // math!
}
