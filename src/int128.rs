use crate::{UnderlyingInt, bits_to_digits};

impl UnderlyingInt for u128 {
    const ZERO: Self = 0;
    const ONE: Self = 1;
    const TEN: Self = 10;
    const MAX_MATISSA: Self = Self::MAX >> Self::META_BITS;
    const MIN_UNDERINT: Self = (1 << 127) | Self::MAX_MATISSA;

    const BITS: u32 = 128;
    const MAX_SCALE: u32 = 36;

    type Signed = i128;

    fn to_signed(self, sign: u8) -> Self::Signed {
        let i = self as i128; // self as mantissa fits 127-bits
        if sign == 0 { i } else { -i }
    }
    fn from_signed(s: Self::Signed) -> (Self, u8) {
        (s.unsigned_abs(), (s < 0) as u8)
    }

    fn as_u32(self) -> u32 {
        self as u32
    }
    fn from_u32(n: u32) -> Self {
        n as Self
    }

    fn leading_zeros(self) -> u32 {
        self.leading_zeros()
    }

    // caller must make sure that no overflow
    fn mul_exp(self, iexp: u32) -> Self {
        self * get_exp(iexp)
    }

    // we check the overflow
    fn checked_mul_exp(self, iexp: u32) -> Option<Self> {
        self.checked_mul(get_exp(iexp))
    }

    fn div_exp(self, iexp: u32) -> Self {
        let n = self + get_exp(iexp) / 2; // no addition overflow. exp is even.

        // SAFETY: self < MAX_MANTISSA, so n fits in 127-bit
        unsafe { div_pow10::bit128::unchecked_div_single_r1b(n, iexp) }
    }

    #[inline]
    fn mul_with_sum_scale(self, right: Self, sum_scale: u32) -> Option<(Self, u32)> {
        if self.leading_zeros() + right.leading_zeros() >= Self::BITS + Self::META_BITS {
            // fast path, keep the code simple
            let p = self * right;
            if sum_scale <= Self::MAX_SCALE {
                Some((p, sum_scale))
            } else {
                Some((p.div_exp(sum_scale - Self::MAX_SCALE), Self::MAX_SCALE))
            }
        } else {
            // full path
            mul_with_sum_scale_full(self, right, sum_scale)
        }
    }

    #[inline]
    fn div_with_scales(self, d: Self, s_scale: u32, d_scale: u32) -> Option<(Self, u32)> {
        let diff_scale = s_scale.saturating_sub(d_scale);
        let max_scale = Self::MAX_SCALE - diff_scale;

        // TODO optimize 64-bit divisor too
        let (mut q, mut r, mut act_scale) = match u32::try_from(d) {
            Ok(d) => div_with_scales_by32(self, d, max_scale),
            Err(_) => div_with_scales_full(self, d, max_scale),
        };

        // increase the scale if d_scale > s_scale
        let min_scale = d_scale.saturating_sub(s_scale);
        if act_scale < min_scale {
            (q, r) = increase_scale(q, r, d, min_scale - act_scale)?;
            act_scale = min_scale;
        }
        // reduce the scale if division exactly
        else if r == 0 && act_scale > min_scale {
            let (q0, red_scale) = reduce_scale(q, act_scale - min_scale);
            q = q0;
            act_scale -= red_scale;
        }

        // round
        if r * 2 >= d {
            if q == u128::MAX_MATISSA {
                // The final scale must be 0 (but why?), there is no room to reduce
                debug_assert_eq!(diff_scale + act_scale - min_scale, 0);
                return None;
            }
            q += 1;
        }

        Some((q, diff_scale + act_scale - min_scale))
    }
}

// the caller must make sure that: @i < 39
fn get_exp(i: u32) -> u128 {
    // Although in most cases, 36 (which is MAX_SCALE) is enough,
    // but in some cases we need more.
    debug_assert!(i < 39);

    unsafe { *ALL_EXPS.get_unchecked(i as usize) }
}

// calculate: n / d and n % d
#[inline]
fn div_rem(n: u128, d: u128) -> (u128, u128) {
    if (n | d) <= u64::MAX as u128 {
        let n64 = n as u64;
        let d64 = d as u64;
        ((n64 / d64) as u128, (n64 % d64) as u128)
    } else {
        (n / d, n % d)
    }
}

// calculate: a * b = (mhigh,mlow)
#[inline]
const fn mul2(a: u128, b: u128) -> (u128, u128) {
    let (ahigh, alow) = (a >> 64, a & u64::MAX as u128);
    let (bhigh, blow) = (b >> 64, b & u64::MAX as u128);

    let (mid, carry1) = (alow * bhigh).overflowing_add(ahigh * blow);
    let (mlow, carry2) = (alow * blow).overflowing_add(mid << 64);
    let mhigh = ahigh * bhigh + (mid >> 64) + ((carry1 as u128) << 64) + carry2 as u128;
    (mhigh, mlow)
}

fn reduce_scale(n: u128, max_scale: u32) -> (u128, u32) {
    if n == 0 {
        return (0, 0);
    }
    if n < 1 << 96 {
        reduce_scale_96(n, max_scale)
    } else {
        reduce_scale_full(n, max_scale)
    }
}

fn reduce_scale_96(mut n: u128, max_scale: u32) -> (u128, u32) {
    let mut left_scale = max_scale;
    while n as u8 == 0 && left_scale >= 8 {
        let (q, r) = div96_by32(n, 10000_0000);
        if r != 0 {
            break;
        }
        n = q;
        left_scale -= 8;
    }
    if n & 0xF == 0 && left_scale >= 4 {
        let (q, r) = div96_by32(n, 10000);
        if r == 0 {
            n = q;
            left_scale -= 4;
        }
    }
    if n & 0x3 == 0 && left_scale >= 2 {
        let (q, r) = div96_by32(n, 100);
        if r == 0 {
            n = q;
            left_scale -= 2;
        }
    }
    if n & 0x1 == 0 && left_scale >= 1 {
        let (q, r) = div96_by32(n, 10);
        if r == 0 {
            n = q;
            left_scale -= 1;
        }
    }
    (n, max_scale - left_scale)
}

fn reduce_scale_full(mut n: u128, max_scale: u32) -> (u128, u32) {
    let mut left_scale = max_scale;
    while n as u8 == 0 && left_scale >= 8 {
        // SAFETY: n < MAX_MANTISSA, so fits in 127-bit
        let q = unsafe { div_pow10::bit128::unchecked_div_single_r1b(n, 8) };
        if (q as u32).wrapping_mul(10000_0000) != n as u32 {
            break;
        }
        n = q;
        left_scale -= 8;
    }
    if n & 0xF == 0 && left_scale >= 4 {
        // SAFETY: n < MAX_MANTISSA, so fits in 127-bit
        let q = unsafe { div_pow10::bit128::unchecked_div_single_r1b(n, 4) };
        if (q as u32).wrapping_mul(10000) == n as u32 {
            n = q;
            left_scale -= 4;
        }
    }
    if n & 0x3 == 0 && left_scale >= 2 {
        // SAFETY: n < MAX_MANTISSA, so fits in 127-bit
        let q = unsafe { div_pow10::bit128::unchecked_div_single_r1b(n, 2) };
        if (q as u32).wrapping_mul(100) == n as u32 {
            n = q;
            left_scale -= 2;
        }
    }
    if n & 0x1 == 0 && left_scale >= 1 {
        // SAFETY: n < MAX_MANTISSA, so fits in 127-bit
        let q = unsafe { div_pow10::bit128::unchecked_div_single_r1b(n, 1) };
        if (q as u32).wrapping_mul(10) == n as u32 {
            n = q;
            left_scale -= 1;
        }
    }
    (n, max_scale - left_scale)
}

#[cold]
fn increase_scale(q: u128, r: u128, d: u128, scale: u32) -> Option<(u128, u128)> {
    let (q2, r2) = div_rem(r.checked_mul_exp(scale)?, d);
    let q = q.checked_mul_exp(scale)?.checked_add(q2)?;
    if q <= u128::MAX_MATISSA {
        Some((q, r2))
    } else {
        None
    }
}

fn mul_with_sum_scale_full(a: u128, b: u128, sum_scale: u32) -> Option<(u128, u32)> {
    let (high, low) = mul2(a, b);

    if high == 0 {
        // the production @low is in range [MAX_MATISSA / 2, u128::MAX]
        return big128_with_sum_scale(low, sum_scale);
    }

    // check the mantissa @high..@low
    //
    // It's hard to calculate how many digits to shrink exactly, so here we
    // get the ceiling value @clear_digits, which may be 1 more than need.
    // The value may be MAX_SCALE+1 at biggest.
    let bits = 128 + u128::META_BITS - high.leading_zeros();
    let mut clear_digits = bits_to_digits(bits) + 1; // +1 for ceiling

    // check the scale @sum_scale
    if sum_scale > u128::MAX_SCALE {
        // normal case
        clear_digits = clear_digits.max(sum_scale - u128::MAX_SCALE);
    } else if clear_digits > sum_scale {
        // edge case, overflow, return None
        if clear_digits == sum_scale + 1 {
            // edge case in edge case. The overflow may be false because
            // the @clear_digits maybe 1 more than need. So here we give
            // it one more chance by decreasing it by 1 and check it later.
            clear_digits -= 1;
        } else {
            return None;
        }
    }

    // prepare for rounding
    let (low, carry) = low.overflowing_add(get_exp(clear_digits) / 2);
    let high = high + carry as u128;

    // SAFETY: high > 10.pow(clear_digits) because of the META_BITS
    let (q, _r) = unsafe { div_pow10::bit128::unchecked_div_double(high, low, clear_digits) };

    // handle the edge case above
    if q > u128::MAX_MATISSA {
        debug_assert_eq!(clear_digits, sum_scale);
        return None;
    }

    Some((q, sum_scale - clear_digits))
}

// reduce the @man to under MAX_MANTISSA
#[cold]
fn big128_with_sum_scale(man: u128, sum_scale: u32) -> Option<(u128, u32)> {
    // check the mantissa @man, which is in range [MAX_MATISSA / 2, u128::MAX]
    let mut clear_digits = if man > u128::MAX_MATISSA * 100 {
        3
    } else if man > u128::MAX_MATISSA * 10 {
        2
    } else if man > u128::MAX_MATISSA {
        1
    } else {
        0
    };

    // check the scale @sum_scale
    if sum_scale > u128::MAX_SCALE {
        clear_digits = clear_digits.max(sum_scale - u128::MAX_SCALE);
    } else if clear_digits > sum_scale {
        return None; // overflow
    }

    // rescale if need
    if clear_digits == 0 {
        Some((man, sum_scale))
    } else {
        // can not call div_exp() because @man is larger than MAX_MANTASSI
        //
        // SAFETY: man > 10.pow(clear_digits) because of the META_BITS
        let mut q = unsafe { div_pow10::bit128::unchecked_div_single(man, clear_digits) };
        let exp = get_exp(clear_digits);
        let r = man - q * exp;
        if r >= exp / 2 {
            q += 1;
        }

        Some((q, sum_scale - clear_digits))
    }
}

fn div_with_scales_by32(n: u128, d: u32, max_scale: u32) -> (u128, u128, u32) {
    let (q, r) = div128_by32(n, d);
    if r == 0 {
        return (q, 0, 0);
    }

    // find the biggest @act_scale that
    // - r * 10.pow(act_scale) fits in u128 (128-bit)
    // - q * 10.pow(act_scale) fits in mantissa (121-bit)
    // - act_scale <= max_scale
    let avail_bits = r.leading_zeros().min(q.leading_zeros() - u128::META_BITS);
    let act_scale = bits_to_digits(avail_bits).min(max_scale);
    if act_scale == 0 {
        return (q, r, 0);
    }

    // We do the division once only, but no loop. So in worst cases (@d is big
    // and @n < @d), the @q may only have 96 significant bits. Although it is
    // not fill the 121 bits, it should be enough.
    let (q2, r2) = div128_by32(r.mul_exp(act_scale), d);
    (q.mul_exp(act_scale) + q2, r2, act_scale)
}

fn div_with_scales_full(n: u128, d: u128, max_scale: u32) -> (u128, u128, u32) {
    let (mut q, mut r) = div_rem(n, d);

    // long division
    let mut act_scale = 0;
    while r != 0 {
        let avail_bits = r.leading_zeros().min(q.leading_zeros() - u128::META_BITS);
        let scale = bits_to_digits(avail_bits).min(max_scale - act_scale);
        if scale == 0 {
            break;
        }

        r = r.mul_exp(scale);
        let q2 = r / d;
        r -= q2 * d;

        q = q.mul_exp(scale) + q2;
        act_scale += scale;
    }

    (q, r, act_scale)
}

fn div96_by32(a: u128, b: u32) -> (u128, u128) {
    let b = b as u64;

    let high = (a >> 64) as u64;
    let low = a as u64;

    let n = high << 32 | low >> 32;
    let q1 = n / b;
    let r = n % b;

    let n = r << 32 | low & u32::MAX as u64;
    let q2 = n / b;
    let r = n % b;

    (((q1 as u128) << 32) | q2 as u128, r as u128)
}

fn div128_by32(a: u128, b: u32) -> (u128, u128) {
    let b = b as u64;

    let high = (a >> 64) as u64;
    let low = a as u64;

    let q0 = high / b;
    let r = high % b;

    let n = r << 32 | low >> 32;
    let q1 = n / b;
    let r = n % b;

    let n = r << 32 | low & u32::MAX as u64;
    let q2 = n / b;
    let r = n % b;

    (
        (q0 as u128) << 64 | ((q1 as u128) << 32) | q2 as u128,
        r as u128,
    )
}

const ALL_EXPS: [u128; 39] = [
    1,
    10_u128.pow(1),
    10_u128.pow(2),
    10_u128.pow(3),
    10_u128.pow(4),
    10_u128.pow(5),
    10_u128.pow(6),
    10_u128.pow(7),
    10_u128.pow(8),
    10_u128.pow(9),
    10_u128.pow(10),
    10_u128.pow(11),
    10_u128.pow(12),
    10_u128.pow(13),
    10_u128.pow(14),
    10_u128.pow(15),
    10_u128.pow(16),
    10_u128.pow(17),
    10_u128.pow(18),
    10_u128.pow(19),
    10_u128.pow(20),
    10_u128.pow(21),
    10_u128.pow(22),
    10_u128.pow(23),
    10_u128.pow(24),
    10_u128.pow(25),
    10_u128.pow(26),
    10_u128.pow(27),
    10_u128.pow(28),
    10_u128.pow(29),
    10_u128.pow(30),
    10_u128.pow(31),
    10_u128.pow(32),
    10_u128.pow(33),
    10_u128.pow(34),
    10_u128.pow(35),
    10_u128.pow(36),
    10_u128.pow(37),
    10_u128.pow(38),
];
