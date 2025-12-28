#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::unreadable_literal
)]

mod args;
mod traits;
#[cfg(test)]
mod verify;

use crate::args::Type;
use anyhow::Result;
use arrayvec::ArrayString;
use lexical_core::FormattedSize;
use rand::SeedableRng as _;
use rand::distr::{Distribution, StandardUniform};
use rand::rngs::SmallRng;
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
        name: "lexical",
        f32: |value, f| {
            let mut buffer = [0u8; f32::FORMATTED_SIZE_DECIMAL];
            let bytes = lexical_core::write(value, &mut buffer);
            f(unsafe { str::from_utf8_unchecked(bytes) });
        },
        f64: |value, f| {
            let mut buffer = [0u8; f64::FORMATTED_SIZE_DECIMAL];
            let bytes = lexical_core::write(value, &mut buffer);
            f(unsafe { str::from_utf8_unchecked(bytes) });
        },
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

fn main() -> Result<()> {
    let data = Data::random(COUNT);
    let mut prev_name = None;

    for (imp, ty) in args::parse()? {
        if prev_name != Some(imp.name) {
            println!("\n{}", imp.name);
            prev_name = Some(imp.name);
        }
        match ty {
            Type::F32 => measure(&data.f32, imp.f32),
            Type::F64 => measure(&data.f64, imp.f64),
        }
    }

    Ok(())
}
