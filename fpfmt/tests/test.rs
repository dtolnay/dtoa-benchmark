#![allow(clippy::float_cmp)]

use rand::rngs::SmallRng;
use rand::{RngCore as _, SeedableRng as _};

const N: usize = if cfg!(miri) {
    500
} else if let b"0" = opt_level::OPT_LEVEL.as_bytes() {
    10_000_000
} else {
    100_000_000
};

#[test]
fn roundtrip() {
    let mut fpfmt_buffer = fpfmt::Buffer::new();
    let mut rng = SmallRng::from_os_rng();
    let mut fail = 0;

    for _ in 0..N {
        let bits = rng.next_u64();
        let float = f64::from_bits(bits);
        if !float.is_finite() {
            continue;
        }
        let fpfmt = fpfmt_buffer.format_finite(float);
        let matches = fpfmt
            .parse::<f64>()
            .is_ok_and(|roundtrip| roundtrip == float);
        if !matches {
            eprintln!("{float:?} FPFMT={fpfmt}");
            fail += 1;
        }
    }

    assert!(fail == 0, "{fail} mismatches");
}
