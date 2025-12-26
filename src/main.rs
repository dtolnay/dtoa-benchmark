#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::unreadable_literal
)]

use arrayvec::ArrayString;
use rand::rngs::SmallRng;
use rand::{RngCore as _, SeedableRng as _};
use std::fmt::Write as _;
use std::hint;
use std::time::{Duration, Instant};

const COUNT: usize = 100_000;
const PASSES: usize = 2;
const TRIALS: usize = 3;

type F = fn(f64, &mut dyn FnMut(&str));

#[derive(Copy, Clone)]
struct Impl {
    name: &'static str,
    dtoa: F,
}

static IMPLS: &[Impl] = &[
    Impl {
        name: "core[Display]",
        dtoa: |value, f| {
            let mut buffer = ArrayString::<327>::new();
            write!(buffer, "{value}").unwrap();
            f(&buffer);
        },
    },
    Impl {
        name: "core[LowerExp]",
        dtoa: |value, f| {
            let mut buffer = ArrayString::<24>::new();
            write!(buffer, "{value:e}").unwrap();
            f(&buffer);
        },
    },
    Impl {
        name: "dtoa",
        dtoa: |value, f| f(dtoa::Buffer::new().format_finite(value)),
    },
    Impl {
        name: "ryu",
        dtoa: |value, f| f(ryu::Buffer::new().format_finite(value)),
    },
    Impl {
        name: "teju",
        dtoa: |value, f| f(teju::Buffer::new().format_finite(value)),
    },
    Impl {
        name: "zmij",
        dtoa: |value, f| f(zmij::Buffer::new().format_finite(value)),
    },
    Impl {
        name: "null",
        dtoa: |_value, f| f(""),
    },
];

fn verify_value(value: f64, f: F) -> usize {
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

fn verify(f: F, name: &str) {
    const VERIFY_RANDOM_COUNT: usize = 100_000;
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

fn verify_all() {
    for imp in IMPLS {
        if imp.name != "null" {
            verify(imp.dtoa, imp.name);
        }
    }
}

struct Data;

impl Data {
    const MAX_DIGIT: usize = 17;

    fn random(count: usize) -> [Vec<f64>; Self::MAX_DIGIT] {
        let mut data = [const { Vec::new() }; Self::MAX_DIGIT];
        let mut rng = SmallRng::seed_from_u64(1);

        for digit in 1..=Self::MAX_DIGIT {
            for _i in 0..count {
                let mut d;
                while {
                    d = f64::from_bits(rng.next_u64());
                    d.is_nan() || d.is_infinite()
                } {}

                // Convert to string with limited digits, and convert it back.
                let buffer = format!("{:.prec$e}", d, prec = digit - 1);
                let roundtrip = buffer.parse().unwrap();

                data[digit - 1].push(roundtrip);
            }
        }

        data
    }
}

fn measure(data: &[Vec<f64>; Data::MAX_DIGIT], f: F, name: &str) {
    println!("\n{name}");

    for (i, vec) in data.iter().enumerate() {
        let mut duration = Duration::MAX;
        for _trial in 0..TRIALS {
            let begin = Instant::now();
            for _pass in 0..PASSES {
                for &value in vec {
                    f(value, &mut |repr| {
                        hint::black_box(repr);
                    });
                }
            }
            duration = Ord::min(duration, begin.elapsed());
        }
        println!(
            "  ({}, {:.2})",
            i + 1,
            duration.as_secs_f64() * 1e9 / (PASSES * vec.len()) as f64,
        );
    }
}

fn main() {
    verify_all();

    let data = Data::random(COUNT);

    for imp in IMPLS {
        measure(&data, imp.dtoa, imp.name);
    }
}
