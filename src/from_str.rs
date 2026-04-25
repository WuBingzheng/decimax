use core::fmt;
use core::str::FromStr;

use crate::{Decimal, UnderlyingInt};

/// Error in converting from string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// Empty string.
    Empty,
    /// Invalid digit in the string.
    Invalid,
    /// Overflow.
    Overflow,
    /// Precision out of range.
    Precision,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Empty => "empty string",
            Self::Invalid => "invalid digit in the string",
            Self::Overflow => "overflow",
            Self::Precision => "precision out of range",
        };
        write!(f, "{s}")
    }
}

impl core::error::Error for ParseError {}

use core::num::{IntErrorKind, ParseIntError};
impl From<ParseIntError> for ParseError {
    fn from(pie: ParseIntError) -> Self {
        match pie.kind() {
            IntErrorKind::Empty => ParseError::Empty,
            IntErrorKind::InvalidDigit => ParseError::Invalid,
            _ => ParseError::Overflow,
        }
    }
}

impl<I, const S: bool> FromStr for Decimal<I, S>
where
    I: UnderlyingInt + FromStr<Err = ParseIntError>,
{
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        if s.is_empty() {
            return Err(ParseError::Empty);
        }

        let (s, sign) = match s.chars().next() {
            Some('-') => {
                if !S {
                    // `-` is invalid for unsigned
                    return Err(ParseError::Invalid);
                }
                (&s[1..], 1)
            }
            Some('+') => (&s[1..], 0),
            _ => (s, 0),
        };

        let (man, scale) = if let Some((int_str, frac_str)) = s.split_once('.') {
            let scale = frac_str.len() as u32;
            if scale > I::MAX_SCALE {
                return Err(ParseError::Precision);
            }

            let int_num = I::from_str(int_str)?;
            let frac_num = I::from_str(frac_str)?;

            let int_part = int_num.checked_mul_exp(scale).ok_or(ParseError::Overflow)?;

            (int_part + frac_num, scale)
        } else {
            (I::from_str(s)?, 0)
        };

        if Self::valid_mantissa(man) {
            Ok(Self::pack(sign, scale, man))
        } else {
            Err(ParseError::Overflow)
        }
    }
}
