use core::fmt;
use core::mem::MaybeUninit;

use crate::{Decimal, UnderlyingInt};

impl<I: UnderlyingInt> fmt::Display for Decimal<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (sign, scale, man) = self.unpack();

        let mut buf: [MaybeUninit<u8>; 80] = [MaybeUninit::uninit(); 80];
        assert!(scale <= 36);

        let offset = display_num(man, scale, f.precision(), &mut buf);

        // SAFETY: offset is updated along with buf
        let buf = unsafe {
            core::slice::from_raw_parts(buf[offset..].as_ptr().cast(), buf.len() - offset)
        };

        // SAFETY: all data is valid charactor
        let s = unsafe { str::from_utf8_unchecked(buf) };

        f.pad_integral(sign == 0 || man == I::ZERO, "", s)
    }
}

fn display_num<I: UnderlyingInt>(
    uns: I,
    scale: u32,
    precision: Option<usize>,
    buf: &mut [MaybeUninit<u8>],
) -> usize {
    let precision = precision.unwrap_or(scale as usize);

    if scale == 0 {
        let mut offset = buf.len();
        if precision > 0 {
            // pad zeros and set point
            offset = pad_zeros(precision, buf);
            offset -= 1;
            buf[offset].write(b'.');
        }
        return dump_single(uns, &mut buf[..offset]);
    }

    let scale = scale as usize;

    if precision >= scale {
        let (int, frac) = uns.div_rem_exp(scale as u32);
        let offset = pad_zeros(precision.min(I::MAX_SCALE as usize) - scale, buf);
        dump_decimal(int, frac, scale, &mut buf[..offset])
    } else {
        let uns = uns.div_exp((scale - precision) as u32);
        if precision == 0 {
            dump_single(uns, buf)
        } else {
            let (int, frac) = uns.div_rem_exp(precision as u32);
            dump_decimal(int, frac, precision, buf)
        }
    }
}

// dump: "int . frac"
fn dump_decimal<I: UnderlyingInt>(
    int: I,
    frac: I,
    scale: usize,
    buf: &mut [MaybeUninit<u8>],
) -> usize {
    let mut offset = dump_single(frac, buf);

    offset = pad_zeros(scale - (buf.len() - offset), &mut buf[..offset]);

    offset -= 1;
    buf[offset].write(b'.');

    dump_single(int, &mut buf[..offset])
}

// dump a single integer number
// This is much faster than using integers' Display.
fn dump_single<I: UnderlyingInt>(n: I, buf: &mut [MaybeUninit<u8>]) -> usize {
    static DECIMAL_PAIRS: &[u8; 200] = b"\
        0001020304050607080910111213141516171819\
        2021222324252627282930313233343536373839\
        4041424344454647484950515253545556575859\
        6061626364656667686970717273747576777879\
        8081828384858687888990919293949596979899";

    let mut offset = buf.len();
    let mut remain = n;

    // Format per two digits from the lookup table.
    while remain >= I::TEN {
        offset -= 2;

        let pair: usize = (remain % I::HUNDRED).as_u32() as usize;
        remain = remain / I::HUNDRED;
        buf[offset + 0].write(DECIMAL_PAIRS[pair * 2 + 0]);
        buf[offset + 1].write(DECIMAL_PAIRS[pair * 2 + 1]);
    }

    // Format the last remaining digit, if any.
    if remain != I::ZERO || n == I::ZERO {
        offset -= 1;
        let remain: u8 = remain.as_u32() as u8;
        buf[offset].write(b'0' + remain);
    }

    offset
}

fn pad_zeros(n: usize, buf: &mut [MaybeUninit<u8>]) -> usize {
    let mut offset = buf.len();
    for _ in 0..n {
        offset -= 1;
        buf[offset].write(b'0');
    }
    offset
}
