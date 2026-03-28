use crate::UnderlyingInt;
use div_pow10::bit128;

const ALL_EXPS: [i128; 39] = [
    1,
    10_i128.pow(1),
    10_i128.pow(2),
    10_i128.pow(3),
    10_i128.pow(4),
    10_i128.pow(5),
    10_i128.pow(6),
    10_i128.pow(7),
    10_i128.pow(8),
    10_i128.pow(9),
    10_i128.pow(10),
    10_i128.pow(11),
    10_i128.pow(12),
    10_i128.pow(13),
    10_i128.pow(14),
    10_i128.pow(15),
    10_i128.pow(16),
    10_i128.pow(17),
    10_i128.pow(18),
    10_i128.pow(19),
    10_i128.pow(20),
    10_i128.pow(21),
    10_i128.pow(22),
    10_i128.pow(23),
    10_i128.pow(24),
    10_i128.pow(25),
    10_i128.pow(26),
    10_i128.pow(27),
    10_i128.pow(28),
    10_i128.pow(29),
    10_i128.pow(30),
    10_i128.pow(31),
    10_i128.pow(32),
    10_i128.pow(33),
    10_i128.pow(34),
    10_i128.pow(35),
    10_i128.pow(36),
    10_i128.pow(37),
    10_i128.pow(38),
];

// maxmax = pow(2, 121) - 1
// for i in range(1,40):
//     print("%d," % (maxmax // pow(10, i)))
const AVAIL_THRESHOLD: [i128; 37] = [
    265845599156983174580761412056068915,
    26584559915698317458076141205606891,
    2658455991569831745807614120560689,
    265845599156983174580761412056068,
    26584559915698317458076141205606,
    2658455991569831745807614120560,
    265845599156983174580761412056,
    26584559915698317458076141205,
    2658455991569831745807614120,
    265845599156983174580761412,
    26584559915698317458076141,
    2658455991569831745807614,
    265845599156983174580761,
    26584559915698317458076,
    2658455991569831745807,
    265845599156983174580,
    26584559915698317458,
    2658455991569831745,
    265845599156983174,
    26584559915698317,
    2658455991569831,
    265845599156983,
    26584559915698,
    2658455991569,
    265845599156,
    26584559915,
    2658455991,
    265845599,
    26584559,
    2658455,
    265845,
    26584,
    2658,
    265,
    26,
    2,
    0,
];

fn get_exp(i: u8) -> i128 {
    debug_assert!(i <= 36);
    unsafe { *ALL_EXPS.get_unchecked(i as usize) }
}

impl UnderlyingInt for i128 {
    const MAX: Self = Self::MAX;
    const MIN: Self = Self::MIN;
    const ZERO: Self = 0;
    const TEN: Self = 10;
    const BITS: u8 = 128;
    const MATISSA_DIGITS: u8 = 36;
    const SCALE_BITS: u8 = 6;
    const MAX_SCALE: u8 = 36;
    const IS_SIGNED: bool = true;

    fn as_u8(self) -> u8 {
        self as u8
    }
    fn from_u8(n: u8) -> Self {
        n as Self
    }

    fn leading_zeros(self) -> u32 {
        self.leading_zeros()
    }

    fn avail_digits(self) -> u8 {
        let n = self.abs() | 1; // make 0 -> 1
        let bits = n.leading_zeros() - Self::SCALE_BITS as u32 - 1; // 1 for sign
        let digits = bits * 1233 >> 12; // math!
        let ath = unsafe { AVAIL_THRESHOLD.get_unchecked(digits as usize) };
        digits as u8 + (&n <= ath) as u8
    }

    fn is_mul_overflow(self, right: Self) -> bool {
        let ua = self.unsigned_abs();
        let ub = right.unsigned_abs();
        ((256 - ua.leading_zeros() - ub.leading_zeros()) as u8) >= 128 - Self::SCALE_BITS - 1
    }

    #[cold]
    fn mul_with_sum_scale(self, right: Self, sum_scale: u8) -> Option<(Self, u8)> {
        let ua = self.unsigned_abs();
        let ub = right.unsigned_abs();
        // if ((256 - ua.leading_zeros() - ub.leading_zeros()) as u8) < 128 - Self::SCALE_BITS - 1 {
        //     let p = self * right;
        //     if sum_scale <= Self::MAX_SCALE {
        //         Some((p, sum_scale))
        //     } else {
        //         let diff = sum_scale - Self::MAX_SCALE;
        //         Some((p.div_exp(diff), Self::MAX_SCALE))
        //     }
        // } else {
        let (high, low) = mul2(ua, ub);
        let extra = (high << (Self::SCALE_BITS + 1)) | (low >> (128 - Self::SCALE_BITS - 1));
        let digits = (128 - extra.leading_zeros()) * 1233 >> 12;
        let mut more_digits = digits as u8 + ((extra > get_exp(digits as u8) as u128) as u8);

        if sum_scale <= Self::MAX_SCALE {
            if more_digits > sum_scale {
                return None;
            }
        } else {
            more_digits = more_digits.max(sum_scale - Self::MAX_SCALE);
        }

        let (q, _) = unsafe { bit128::unchecked_div_double(high, low, more_digits as u32) };

        let q = q as i128;

        let q = if self ^ right < 0 { -q } else { q };
        Some((q, sum_scale - more_digits))
        // }
    }

    // caller must make sure that no overflow
    fn mul_exp(self, iexp: u8) -> Self {
        self * get_exp(iexp)
    }

    #[inline] // TODO good for mac, bad for linux
    fn div_exp(self, iexp: u8) -> Self {
        let exp = get_exp(iexp);
        let n = self.unsigned_abs() + exp as u128 / 2; // no addition overflow. exp is even.
        let q = unsafe { bit128::unchecked_div_single_r1b(n, iexp as u32) } as i128;
        if self >= 0 { q } else { -q }
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
