use crate::UnderlyingInt;

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

impl UnderlyingInt for i128 {
    const MAX: Self = Self::MAX;
    const MIN: Self = Self::MIN;
    const ZERO: Self = 0;
    const TEN: Self = 10;
    const BITS: u8 = 128;
    const MATISSA_DIGITS: u8 = 36;
    const SCALE_BITS: u8 = 6;
    const MAX_SCALE: u8 = 36;

    fn as_u8(self) -> u8 {
        self as u8
    }
    fn from_u8(n: u8) -> Self {
        n as Self
    }

    fn get_exp(i: u8) -> i128 {
        unsafe { *ALL_EXPS.get_unchecked(i as usize) }
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

    fn mul_div_exp(self, right: Self, iexp: u8) -> Self {
        todo!()
    }
}
