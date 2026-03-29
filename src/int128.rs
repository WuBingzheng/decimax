use crate::{UnderlyingInt, bits_to_digits};

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

fn get_exp(i: u32) -> u128 {
    debug_assert!(i <= 36);
    unsafe { *ALL_EXPS.get_unchecked(i as usize) }
}

impl UnderlyingInt for u128 {
    const ZERO: Self = 0;
    const ONE: Self = 1;
    const TEN: Self = 10;
    const MAX: Self = Self::MAX;
    const MIN: Self = Self::MIN;

    const BITS: u32 = 128;
    const MAX_SCALE: u32 = 36;
    const MAX_MATISSA: Self = Self::MAX >> (Self::SCALE_BITS + 1);

    fn as_u32(self) -> u32 {
        self as u32
    }
    fn from_u32(n: u32) -> Self {
        n as Self
    }

    fn leading_zeros(self) -> u32 {
        self.leading_zeros()
    }

    fn mul_with_sum_scale(self, right: Self, sum_scale: u32) -> Option<(Self, u32)> {
        let (high, low) = mul2(self, right);

        // Calculate the digits that needed be cleared.
        // We do not calculate it accurately for performance.
        // The clear_digits may be bigger than needed.
        let clear_bits = 128 + Self::SCALE_BITS + 1 - high.leading_zeros(); // ignore @low
        let mut clear_digits = bits_to_digits(clear_bits) + 1; // +1 for ceiling

        // Reduce sum_scale if too big.
        if sum_scale > Self::MAX_SCALE {
            clear_digits = clear_digits.max(sum_scale - Self::MAX_SCALE);
        }

        // overflow
        if clear_digits > sum_scale {
            return None;
        }

        let (q, _) = unsafe { div_pow10::bit128::unchecked_div_double(high, low, clear_digits) };
        Some((q, sum_scale - clear_digits))
    }

    // caller must make sure that no overflow
    fn mul_exp(self, iexp: u32) -> Self {
        self * get_exp(iexp)
    }

    fn div_exp(self, iexp: u32) -> Self {
        let n = self + get_exp(iexp) / 2; // no addition overflow. exp is even.
        unsafe { div_pow10::bit128::unchecked_div_single_r1b(n, iexp) }
    }

    fn div_rem_exp(self, iexp: u32) -> (Self, Self) {
        let q = unsafe { div_pow10::bit128::unchecked_div_single_r1b(self, iexp) };
        let r = self - q * get_exp(iexp);
        (q, r)
    }

    // self * 2^extra_scale / right
    fn div_with_extra_scale(self, right: Self, mut extra_scale: u32) -> Option<(Self, Self)> {
        if self < INVERSES[extra_scale as usize] {
            // self * 2^extra_scale is not overflow
            Some((self * get_exp(extra_scale)).div_rem(right))
        } else {
            let mut q = 0_u128;
            let mut r = self;
            loop {
                let avail_digits = bits_to_digits(r.leading_zeros());
                let iexp = avail_digits.min(extra_scale);
                let exp = get_exp(iexp);
                let n = r * exp;
                q = q.checked_mul(exp)?.checked_add(n / right)?;
                r = n % right;
                extra_scale -= iexp;
                if extra_scale == 0 {
                    break;
                }
            }
            Some((q, r))
        }
    }

    fn div_with_avail_scale(self, right: Self, avail_scale: u32) -> (Self, Self, u32) {
        let mut q = 0_u128;
        let mut r = self;
        let mut left_scale = avail_scale;
        loop {
            let scale = bits_to_digits(r.leading_zeros()).min(left_scale);
            let (q1, r1) = r.mul_exp(scale).div_rem(right);

            q = q.mul_exp(scale) + q1;
            r = r1;
            left_scale -= scale;

            if left_scale == 0 || r == 0 {
                break;
            }
        }

        // try best to reduce the scale
        let (q, red_scale) = reduce_scale(q);

        (q, r, avail_scale - left_scale - red_scale)
    }

    #[inline]
    fn div_rem(self, right: Self) -> (Self, Self) {
        if (self | right) <= u64::MAX as u128 {
            let n64 = self as u64;
            let d64 = right as u64;
            ((n64 / d64) as u128, (n64 % d64) as u128)
        } else {
            (self / right, self % right)
        }
    }
}

// calculate: a * b = (mhigh,mlow)
const fn mul2(a: u128, b: u128) -> (u128, u128) {
    let (ahigh, alow) = (a >> 64, a & u64::MAX as u128);
    let (bhigh, blow) = (b >> 64, b & u64::MAX as u128);

    let (mid, carry1) = (alow * bhigh).overflowing_add(ahigh * blow);
    let (mlow, carry2) = (alow * blow).overflowing_add(mid << 64);
    let mhigh = ahigh * bhigh + (mid >> 64) + ((carry1 as u128) << 64) + carry2 as u128;
    (mhigh, mlow)
}

fn reduce_scale(mut n: u128) -> (u128, u32) {
    if n == 0 {
        return (0, 0);
    }
    let mut scale = 0;
    while n.trailing_zeros() >= 8 {
        let q = unsafe { div_pow10::bit128::unchecked_div_single_r1b(n, 8) };
        if q * get_exp(8) != n {
            break;
        }
        n = q;
        scale += 8;
    }
    if n.trailing_zeros() >= 4 {
        let q = unsafe { div_pow10::bit128::unchecked_div_single_r1b(n, 4) };
        if q * get_exp(4) == n {
            n = q;
            scale += 4;
        }
    }
    if n.trailing_zeros() >= 2 {
        let q = unsafe { div_pow10::bit128::unchecked_div_single_r1b(n, 2) };
        if q * get_exp(2) == n {
            n = q;
            scale += 2;
        }
    }
    if n.trailing_zeros() >= 1 {
        let q = unsafe { div_pow10::bit128::unchecked_div_single_r1b(n, 1) };
        if q * get_exp(1) == n {
            n = q;
            scale += 1;
        }
    }
    (n, scale)
}

const INVERSES: [u128; 39] = [
    340282366920938463463374607431768211455,
    34028236692093846346337460743176821145,
    3402823669209384634633746074317682114,
    340282366920938463463374607431768211,
    34028236692093846346337460743176821,
    3402823669209384634633746074317682,
    340282366920938463463374607431768,
    34028236692093846346337460743176,
    3402823669209384634633746074317,
    340282366920938463463374607431,
    34028236692093846346337460743,
    3402823669209384634633746074,
    340282366920938463463374607,
    34028236692093846346337460,
    3402823669209384634633746,
    340282366920938463463374,
    34028236692093846346337,
    3402823669209384634633,
    340282366920938463463,
    34028236692093846346,
    3402823669209384634,
    340282366920938463,
    34028236692093846,
    3402823669209384,
    340282366920938,
    34028236692093,
    3402823669209,
    340282366920,
    34028236692,
    3402823669,
    340282366,
    34028236,
    3402823,
    340282,
    34028,
    3402,
    340,
    34,
    3,
];
