#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::unreadable_literal
)]

mod traits;

use arrayvec::ArrayString;
use rand::distr::{Distribution, StandardUniform};
use rand::rngs::SmallRng;
use rand::{RngCore as _, SeedableRng as _};
use std::any;
use std::fmt::Write as _;
use std::hint;
use std::time::{Duration, Instant};

const COUNT: usize = if cfg!(miri) { 10 } else { 100_000 };
const TRIALS: usize = if cfg!(miri) { 1 } else { 3 };
const PASSES: usize = if cfg!(miri) { 1 } else { 12 };

type F<T> = fn(T, &mut dyn FnMut(&str));

#[derive(Copy, Clone)]
struct Impl {
    name: &'static str,
    f32: F<f32>,
    f64: F<f64>,
}

static IMPLS: &[Impl] = &[
    Impl {
        name: "core[Display]",
        f32: |value, f| {
            let mut buffer = ArrayString::<327>::new();
            write!(buffer, "{value}").unwrap();
            f(&buffer);
        },
        f64: |value, f| {
            let mut buffer = ArrayString::<327>::new();
            write!(buffer, "{value}").unwrap();
            f(&buffer);
        },
    },
    Impl {
        name: "core[LowerExp]",
        f32: |value, f| {
            let mut buffer = ArrayString::<24>::new();
            write!(buffer, "{value:e}").unwrap();
            f(&buffer);
        },
        f64: |value, f| {
            let mut buffer = ArrayString::<24>::new();
            write!(buffer, "{value:e}").unwrap();
            f(&buffer);
        },
    },
    Impl {
        name: "dtoa",
        f32: |value, f| f(dtoa::Buffer::new().format_finite(value)),
        f64: |value, f| f(dtoa::Buffer::new().format_finite(value)),
    },
    Impl {
        name: "ryu",
        f32: |value, f| f(ryu::Buffer::new().format_finite(value)),
        f64: |value, f| f(ryu::Buffer::new().format_finite(value)),
    },
    #[cfg(not(miri))] // https://github.com/andrepd/teju-jagua-rs/issues/1
    Impl {
        name: "teju",
        f32: |value, f| f(teju::Buffer::new().format_finite(value)),
        f64: |value, f| f(teju::Buffer::new().format_finite(value)),
    },
    Impl {
        name: "zmij",
        f32: |value, f| f(zmij::Buffer::new().format_finite(value)),
        f64: |value, f| f(zmij::Buffer::new().format_finite(value)),
    },
    Impl {
        name: "null",
        f32: |_value, f| f(""),
        f64: |_value, f| f(""),
    },
];

fn verify_value(value: f64, f: F<f64>) -> usize {
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

fn verify(f: F<f64>, name: &str) {
    const VERIFY_RANDOM_COUNT: usize = if cfg!(miri) { 40 } else { 100_000 };
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
            verify(imp.f64, imp.name);
        }
    }
}

struct Data {
    f32: [Vec<f32>; 9],
    f64: [Vec<f64>; 17],
}

impl Data {
    fn random(count: usize) -> Self {
        let mut rng = SmallRng::seed_from_u64(1);
        let mut data = Data {
            f32: [const { Vec::new() }; 9],
            f64: [const { Vec::new() }; 17],
        };
        fill(&mut rng, &mut data.f32, count);
        fill(&mut rng, &mut data.f64, count);
        data
    }
}

fn fill<T, const N: usize>(rng: &mut SmallRng, data: &mut [Vec<T>; N], count: usize)
where
    T: traits::Float,
    StandardUniform: Distribution<T::Bits>,
{
    for (prec, vec) in data.iter_mut().enumerate() {
        vec.reserve_exact(count);
        for _i in 0..count {
            let mut d;
            while {
                let bits = StandardUniform.sample(rng);
                d = T::from_bits(bits);
                !d.is_finite()
            } {}

            // Convert to string with limited digits, and convert it back.
            let buffer = format!("{d:.prec$e}");
            let roundtrip = buffer.parse().unwrap();
            vec.push(roundtrip);
        }
    }
}

fn measure<T, const N: usize>(data: &[Vec<T>; N], f: F<T>)
where
    T: traits::Float,
{
    println!("  {}", any::type_name::<T>());
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
            "    ({}, {:.2})",
            i + 1,
            duration.as_secs_f64() * 1e9 / (PASSES * vec.len()) as f64,
        );
    }
}

fn main() {
    verify_all();

    let data = Data::random(COUNT);

    for imp in IMPLS {
        println!("\n{}", imp.name);
        measure(&data.f32, imp.f32);
        measure(&data.f64, imp.f64);
    }
}
