use crate::UnderlyingInt;

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
        let mut clear_digits = (clear_bits * 1233 >> 12) + 1; // +1 for ceiling

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
