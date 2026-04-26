use crate::{Dec32, Dec64, Dec128, Decimal, UDec32, UDec64, UDec128};

// from integers to signed decimal
macro_rules! convert_signed_from_int {
    ($decimal_int:ty, $decimal_signed:ty; $($from_int:ty),*) => {$(
        impl From<$from_int> for Decimal<$decimal_int, true> {
            fn from(value: $from_int) -> Self {
                Self::from_parts(value as $decimal_signed, 0)
            }
        }
    )*};
}
convert_signed_from_int!(u128, i128; i8, u8, i16, u16, i32, u32, i64, u64);
convert_signed_from_int!(u64, i64; i8, u8, i16, u16, i32, u32);
convert_signed_from_int!(u32, i32; i8, u8, i16, u16);

// from integers to unsigned decimal
macro_rules! convert_unsigned_from_int {
    ($decimal_int:ty; $($from_int:ty),*) => {$(
        impl From<$from_int> for Decimal<$decimal_int, false> {
            fn from(value: $from_int) -> Self {
                Self::from_parts(value as $decimal_int, 0)
            }
        }
    )*};
}
convert_unsigned_from_int!(u128; u8, u16, u32, u64);
convert_unsigned_from_int!(u64; u8, u16, u32);
convert_unsigned_from_int!(u32; u8, u16);

// from between decimals
macro_rules! convert_from {
    ($into_dec:ty, $from_dec:ty) => {
        impl From<$from_dec> for $into_dec {
            fn from(value: $from_dec) -> Self {
                let (sign, scale, man) = value.unpack();
                Self::pack(sign, scale, man.into())
            }
        }
    };
}
convert_from!(Dec128, Dec64);
convert_from!(Dec128, UDec64);
convert_from!(Dec128, Dec32);
convert_from!(Dec128, UDec32);
convert_from!(Dec64, Dec32);
convert_from!(Dec64, UDec32);

convert_from!(UDec128, UDec64);
convert_from!(UDec128, UDec32);
convert_from!(UDec64, UDec32);

// try from between decimals
macro_rules! convert_try_from {
    ($into_dec:ty, $into_int:ty, $from_dec:ty) => {
        impl TryFrom<$from_dec> for $into_dec {
            type Error = ();
            fn try_from(value: $from_dec) -> Result<Self, Self::Error> {
                let (man, scale) = value.parts();
                let man = <$into_int>::try_from(man).map_err(|_| ())?;
                Self::try_from_parts(man, scale).ok_or(())
            }
        }
    };
}

// unsigned <- signed
convert_try_from!(UDec128, u128, Dec128);
convert_try_from!(UDec128, u128, Dec64);
convert_try_from!(UDec128, u128, Dec32);
convert_try_from!(UDec64, u64, Dec128);
convert_try_from!(UDec64, u64, Dec64);
convert_try_from!(UDec64, u64, Dec32);
convert_try_from!(UDec32, u32, Dec128);
convert_try_from!(UDec32, u32, Dec64);
convert_try_from!(UDec32, u32, Dec32);

// signed <- unsigned
convert_try_from!(Dec128, i128, UDec128);
convert_try_from!(Dec64, i64, UDec64);
convert_try_from!(Dec64, i64, UDec128);
convert_try_from!(Dec32, i32, UDec32);
convert_try_from!(Dec32, i32, UDec64);
convert_try_from!(Dec32, i32, UDec128);

// short <- long, same sign
convert_try_from!(Dec64, i64, Dec128);
convert_try_from!(Dec32, i32, Dec64);
convert_try_from!(Dec32, i32, Dec128);
convert_try_from!(UDec64, u64, UDec128);
convert_try_from!(UDec32, u32, UDec64);
convert_try_from!(UDec32, u32, UDec128);
