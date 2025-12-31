#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::unreadable_literal
)]

mod args;
mod data;
mod traits;
#[cfg(test)]
mod verify;

use crate::args::Type;
use crate::data::{Data, DataForType};
use anyhow::Result;
use arrayvec::ArrayString;
use lexical_core::FormattedSize;
use std::any;
use std::fmt::Write as _;
use std::hint;
use std::time::{Duration, Instant};

const COUNT: usize = if cfg!(miri) { 10 } else { 100_000 };
const TRIALS: usize = if cfg!(miri) { 1 } else { 4 };
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

fn measure<T, const N: usize>(data: &DataForType<T, N>, f: F<T>)
where
    T: traits::Float,
{
    println!("  {}", any::type_name::<T>());
    let baseline = if data.unpredictable {
        measure_once(&data.mixed, f)
    } else {
        Duration::ZERO
    };
    for (i, vec) in data.by_precision.iter().enumerate() {
        let duration = measure_once(vec, f).saturating_sub(baseline);
        println!(
            "    ({}, {:.2})",
            i + 1,
            duration.as_secs_f64() * 1e9 / (PASSES * data.count) as f64,
        );
    }
}

fn measure_once<T>(data: &[T], f: F<T>) -> Duration
where
    T: traits::Float,
{
    let mut duration = Duration::MAX;
    for _trial in 0..TRIALS {
        let begin = Instant::now();
        for _pass in 0..PASSES {
            for &value in data {
                f(value, &mut |repr| {
                    hint::black_box(repr);
                });
            }
        }
        duration = Ord::min(duration, begin.elapsed());
    }
    duration
}

fn main() -> Result<()> {
    let args = args::parse()?;
    let data = Data::random(COUNT, args.unpredictable);
    let mut prev_name = None;

    for (imp, ty) in args.benchmark {
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
