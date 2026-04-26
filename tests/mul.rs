use lean_decimal::{Dec32, Dec64, Dec128};

#[test]
fn mul128() {
    // normal
    let a = Dec128::from_parts(100, 2);
    let b = Dec128::from_parts(300, 3);
    let r = Dec128::from_parts(30000, 5);
    assert_eq!(a * b, r);

    // sum of scale overflow
    let a = Dec128::from_parts(100, 12);
    let b = Dec128::from_parts(300, 21);
    let r = Dec128::from_parts(300, 31);
    assert_eq!(a * b, r);

    // product of mantissas overflow of mantissa(121 bit) but
    // fits in u128 (128 bit)
    let a = Dec128::from_parts(1_i128 << 60, 6);
    let b = Dec128::from_parts(1_i128 << 63, 8);
    let r = Dec128::from_parts(((1_i128 << 123) + 5) / 10, 6 + 8 - 1);
    assert_eq!(a * b, r);

    // product of mantissas overflow
    let a = Dec128::from_parts(1_i128 << 90, 12);
    let b = Dec128::from_parts(1_i128 << 90, 14);
    let r = Dec128::from_parts(1532495540865888858358347027150309184_i128, 12 + 14 - 18);
    assert_eq!(a * b, r);

    // overflow
    let a = Dec128::from_parts(1_i128 << 90, 12);
    let b = Dec128::from_parts(1_i128 << 90, 4);
    assert!(a.checked_mul(b).is_none());
}

#[test]
fn mul64() {
    // normal
    let a = Dec64::from_parts(100, 2);
    let b = Dec64::from_parts(300, 3);
    let r = Dec64::from_parts(30000, 5);
    assert_eq!(a * b, r);

    // sum of scale overflow
    let a = Dec64::from_parts(100, 2);
    let b = Dec64::from_parts(300, 15);
    let r = Dec64::from_parts(300, 15);
    assert_eq!(a * b, r);

    // product of mantissas overflow of mantissa(121 bit) but
    // fits in u64 (64 bit)
    let a = Dec64::from_parts(1_i64 << 29, 6);
    let b = Dec64::from_parts(1_i64 << 31, 8);
    let r = Dec64::from_parts(((1_i64 << 60) + 5) / 10, 6 + 8 - 1);
    assert_eq!(a * b, r);

    // product of mantissas overflow
    let a = Dec64::from_parts(1_i64 << 45, 12);
    let b = Dec64::from_parts(1_i64 << 45, 14);
    let r = Dec64::from_parts(12379400392853803_i64, 15);
    assert_eq!(a * b, r);

    // overflow
    let a = Dec64::from_parts(1_i64 << 45, 5);
    let b = Dec64::from_parts(1_i64 << 45, 4);
    assert!(a.checked_mul(b).is_none());
}

#[test]
fn mul32() {
    // normal
    let a = Dec32::from_parts(100, 2);
    let b = Dec32::from_parts(300, 3);
    let r = Dec32::from_parts(30000, 5);
    assert_eq!(a * b, r);

    // sum of scale overflow
    let a = Dec32::from_parts(100, 2);
    let b = Dec32::from_parts(300, 7);
    let r = Dec32::from_parts(300, 7);
    assert_eq!(a * b, r);

    // product of mantissas overflow of mantissa but
    // fits in u32 (32 bit)
    let a = Dec32::from_parts(1_i32 << 14, 2);
    let b = Dec32::from_parts(1_i32 << 16, 4);
    let r = Dec32::from_parts(((1_i32 << 30) + 5) / 10, 2 + 4 - 1);
    assert_eq!(a * b, r);

    // product of mantissas overflow
    let a = Dec32::from_parts(1_i32 << 15, 2);
    let b = Dec32::from_parts(1_i32 << 18, 5);
    let r = Dec32::from_parts(85899346_i32, 5);
    assert_eq!(a * b, r);

    // overflow
    let a = Dec32::from_parts(1_i32 << 25, 2);
    let b = Dec32::from_parts(1_i32 << 22, 1);
    assert!(a.checked_mul(b).is_none());
}
