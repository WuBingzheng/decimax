use crate::{UnderlyingInt, bits_to_digits};

impl UnderlyingInt for u32 {
    const ZERO: Self = 0;
    const ONE: Self = 1;
    const TEN: Self = 10;
    const HUNDRED: Self = 100;
    const MAX: Self = Self::MAX;

    const UNSIGNED_MAX_MATISSA: Self = Self::MAX >> Self::SCALE_BITS;
    const SIGNED_MAX_MATISSA: Self = Self::UNSIGNED_MAX_MATISSA >> 1;
    const SIGNED_MIN_UNDERINT: Self = (1 << (Self::BITS - 1)) | Self::SIGNED_MAX_MATISSA;

    const BITS: u32 = 32;
    const SCALE_BITS: u32 = 3;

    type Signed = i32;

    fn to_signed(self, sign: u8) -> Self::Signed {
        let i = self as i32; // self as mantissa fits 63-bits
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
        let exp = get_exp(iexp);
        let n = self + exp / 2; // no addition overflow. exp is even.
        n / exp
    }

    fn div_rem_exp(self, iexp: u32) -> (Self, Self) {
        div_rem(self, get_exp(iexp))
    }

    #[inline]
    fn mul_with_sum_scale<const S: bool>(self, right: Self, sum_scale: u32) -> Option<(Self, u32)> {
        if self.leading_zeros() + right.leading_zeros() >= Self::BITS + meta_bits::<S>() {
            // fast path, keep the code simple
            let p = self * right;
            if sum_scale <= Self::MAX_SCALE {
                Some((p, sum_scale))
            } else {
                Some((p.div_exp(sum_scale - Self::MAX_SCALE), Self::MAX_SCALE))
            }
        } else {
            // full path
            mul_with_sum_scale_full::<S>(self, right, sum_scale)
        }
    }

    #[inline]
    fn div_with_scales<const S: bool>(
        self,
        d: Self,
        s_scale: u32,
        d_scale: u32,
    ) -> Option<(Self, u32)> {
        let diff_scale = s_scale.saturating_sub(d_scale);
        let max_scale = Self::MAX_SCALE - diff_scale;

        let (mut q, mut r, mut act_scale) = div_with_scales_full::<S>(self, d, max_scale);

        // increase the scale if d_scale > s_scale
        let min_scale = d_scale.saturating_sub(s_scale);
        if act_scale < min_scale {
            (q, r) = increase_scale::<S>(q, r, d, min_scale - act_scale)?;
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
            if q == max_mantissa::<S>() {
                // The final scale must be 0 (but why?), there is no room to reduce
                debug_assert_eq!(diff_scale + act_scale - min_scale, 0);
                return None;
            }
            q += 1;
        }

        Some((q, diff_scale + act_scale - min_scale))
    }
}

fn max_mantissa<const S: bool>() -> u32 {
    if S {
        u32::SIGNED_MAX_MATISSA
    } else {
        u32::UNSIGNED_MAX_MATISSA
    }
}
fn meta_bits<const S: bool>() -> u32 {
    if S {
        u32::SCALE_BITS + 1
    } else {
        u32::SCALE_BITS
    }
}

// the caller must make sure that: @i < 10
fn get_exp(i: u32) -> u32 {
    // Although in most cases, 7 (which is MAX_SCALE) is enough,
    // but in some cases we need more.
    debug_assert!(i < 10);

    unsafe { *ALL_EXPS.get_unchecked(i as usize) }
}

// calculate: n / d and n % d
#[inline]
fn div_rem(n: u32, d: u32) -> (u32, u32) {
    (n / d, n % d)
}

fn reduce_scale(mut n: u32, max_scale: u32) -> (u32, u32) {
    if n == 0 {
        return (0, 0);
    }

    let mut left_scale = max_scale;
    while n & 0x3 == 0 && left_scale >= 2 {
        let (q, r) = div_rem(n, 100);
        if r != 0 {
            break;
        }
        n = q;
        left_scale -= 2;
    }
    if n & 0x1 == 0 && left_scale >= 1 {
        let (q, r) = div_rem(n, 10);
        if r == 0 {
            n = q;
            left_scale -= 1;
        }
    }
    (n, max_scale - left_scale)
}

#[cold]
fn increase_scale<const S: bool>(q: u32, r: u32, d: u32, scale: u32) -> Option<(u32, u32)> {
    let (q2, r2) = div_rem(r.checked_mul_exp(scale)?, d);
    let q = q.checked_mul_exp(scale)?.checked_add(q2)?;
    if q <= max_mantissa::<S>() {
        Some((q, r2))
    } else {
        None
    }
}

fn mul_with_sum_scale_full<const S: bool>(a: u32, b: u32, sum_scale: u32) -> Option<(u32, u32)> {
    let p = a as u64 * b as u64;

    // It's hard to calculate how many digits to shrink exactly, so here we
    // get the ceiling value @clear_digits, which may be 1 more than need.
    // The value may be MAX_SCALE+1 at biggest.
    let bits = 32 + meta_bits::<S>() - p.leading_zeros();
    let mut clear_digits = bits_to_digits(bits) + 1; // +1 for ceiling

    // check the scale @sum_scale
    if sum_scale > u32::MAX_SCALE {
        clear_digits = clear_digits.max(sum_scale - u32::MAX_SCALE);
    }
    if clear_digits > sum_scale {
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
    let q = ((p + get_exp(clear_digits) as u64 / 2) / b as u64) as u32;

    // handle the edge case above
    if q > max_mantissa::<S>() {
        debug_assert_eq!(clear_digits, sum_scale);
        return None;
    }

    Some((q, sum_scale - clear_digits))
}

fn div_with_scales_full<const S: bool>(n: u32, d: u32, max_scale: u32) -> (u32, u32, u32) {
    let (q, r) = div_rem(n, d);
    if r == 0 {
        return (q, 0, 0);
    }

    let act_scale = bits_to_digits(q.leading_zeros() - meta_bits::<S>()).min(max_scale);
    let exp = get_exp(act_scale);

    let n = r as u64 * get_exp(act_scale) as u64;
    let q2 = (n / d as u64) as u32;
    let r2 = (n % d as u64) as u32;

    (q * exp + q2, r2, act_scale)
}

const ALL_EXPS: [u32; 10] = [
    1,
    10_u32.pow(1),
    10_u32.pow(2),
    10_u32.pow(3),
    10_u32.pow(4),
    10_u32.pow(5),
    10_u32.pow(6),
    10_u32.pow(7),
    10_u32.pow(8),
    10_u32.pow(9),
];
