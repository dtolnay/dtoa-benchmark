use rand::rngs::SmallRng;
use rand::{RngCore as _, SeedableRng as _};

const VERIFY_RANDOM_COUNT: usize = if cfg!(miri) { 40 } else { 100_000 };

fn verify_value(value: f64, f: crate::F<f64>) -> usize {
    let mut len = 0;

    f(value, &mut |actual| {
        let Ok(roundtrip) = actual.parse::<f64>() else {
            eprintln!("Error: failed to parse {actual}");
            return;
        };

        if value != roundtrip {
            eprintln!("Error: roundtrip fail {value} -> {actual:?} -> {roundtrip}");
        }

        len = actual.len();
    });

    len
}

fn verify(f: crate::F<f64>, name: &str) {
    print!("Verifying {name:20} ... ");

    // Boundary and simple cases
    verify_value(0.0, f);
    verify_value(0.1, f);
    verify_value(0.12, f);
    verify_value(0.123, f);
    verify_value(0.1234, f);
    verify_value(1.2345, f);
    verify_value(1.0 / 3.0, f);
    verify_value(2.0 / 3.0, f);
    verify_value(10.0 / 3.0, f);
    verify_value(20.0 / 3.0, f);
    verify_value(f64::MIN, f);
    verify_value(f64::MAX, f);
    verify_value(0.0f64.next_up(), f);

    let mut r = SmallRng::seed_from_u64(1);

    let mut len_sum = 0u64;
    let mut len_max = 0usize;
    for _i in 0..VERIFY_RANDOM_COUNT {
        let mut d;
        while {
            d = f64::from_bits(r.next_u64());
            d.is_nan() || d.is_infinite()
        } {}
        let len = verify_value(d, f);
        len_sum += len as u64;
        len_max = usize::max(len_max, len);
    }

    let len_avg = len_sum as f64 / VERIFY_RANDOM_COUNT as f64;
    println!("OK. Length Avg = {len_avg:.3}, Max = {len_max}");
}

#[test]
fn verify_all() {
    for imp in crate::IMPLS {
        if imp.name != "null"
            && let Some(f) = imp.f64
        {
            verify(f, imp.name);
        }
    }
}
