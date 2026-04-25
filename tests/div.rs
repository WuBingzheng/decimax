use lean_decimal::Dec128;

#[test]
fn div_base() {
    // divide by 0
    let a = Dec128::from_parts(100, 2);
    assert!(a.checked_div(Dec128::ZERO).is_none());

    // small
    for n_man_exp in [0, 4, 10, 20, 31] {
        let n_man = 10_i128.pow(n_man_exp);
        for n_scale in [0, 4, 10, 20, 31] {
            for d_man in [1, 9, 59, 99, 127] {
                for d_scale in [0, 4, 10, 25, 31] {
                    let n = Dec128::from_parts(n_man, n_scale);
                    let d = Dec128::from_parts(d_man, d_scale);
                    // println!("{:?} / {:?} =", n, d);
                    if let Some(q) = n.checked_div(d) {
                        // println!("  {:?}", q);
                        let p = d * q;
                        let diff = n - p;
                        // println!("  {:?} {:?}", p, diff);
                        assert!(diff.abs() * 2 <= d);
                    }
                }
            }
        }
    }
}
