#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::unreadable_literal
)]

use arrayvec::ArrayString;
use std::fmt::Write as _;
use std::hint;
use std::sync::OnceLock;
use std::time::Instant;

const VERIFY_RANDOM_COUNT: usize = 100_000;
const ITERATION_PER_DIGIT: usize = 2;
const TRIAL: usize = 3;

type F = fn(f64, &mut dyn FnMut(&str));

#[derive(Copy, Clone)]
struct Test {
    fname: &'static str,
    dtoa: F,
}

static TESTS: &[Test] = &[
    Test {
        fname: "core[Display]",
        dtoa: |value, f| {
            let mut buffer = ArrayString::<327>::new();
            write!(buffer, "{value}").unwrap();
            f(&buffer);
        },
    },
    Test {
        fname: "core[LowerExp]",
        dtoa: |value, f| {
            let mut buffer = ArrayString::<24>::new();
            write!(buffer, "{value:e}").unwrap();
            f(&buffer);
        },
    },
    Test {
        fname: "dtoa",
        dtoa: |value, f| f(dtoa::Buffer::new().format_finite(value)),
    },
    Test {
        fname: "ryu",
        dtoa: |value, f| f(ryu::Buffer::new().format_finite(value)),
    },
    Test {
        fname: "teju",
        dtoa: |value, f| f(teju::Buffer::new().format_finite(value)),
    },
    Test {
        fname: "zmij",
        dtoa: |value, f| f(zmij::Buffer::new().format_finite(value)),
    },
    Test {
        fname: "null",
        dtoa: |_value, f| f(""),
    },
];

struct Random {
    seed: u32,
}

impl Random {
    pub fn new() -> Self {
        Random { seed: 0 }
    }

    pub fn get(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(214013).wrapping_add(531011);
        self.seed
    }
}

fn verify_value(value: f64, f: F, expect: Option<&str>) -> usize {
    let mut len = 0;

    f(value, &mut |actual| {
        if let Some(expect) = expect
            && actual != expect
        {
            eprintln!("Error: expect {expect} but actual {actual}");
        }

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

fn verify(f: F, fname: &str) {
    print!("Verifying {fname:20} ... ");

    // Boundary and simple cases
    // This gives benign errors in ostringstream and sprintf:
    // Error: expect 0.1 but actual 0.10000000000000001
    // Error: expect 1.2345 but actual 1.2344999999999999
    verify_value(0.0, f, None);
    verify_value(0.1, f, Some("0.1"));
    verify_value(0.12, f, Some("0.12"));
    verify_value(0.123, f, Some("0.123"));
    verify_value(0.1234, f, Some("0.1234"));
    verify_value(1.2345, f, Some("1.2345"));
    verify_value(1.0 / 3.0, f, None);
    verify_value(2.0 / 3.0, f, None);
    verify_value(10.0 / 3.0, f, None);
    verify_value(20.0 / 3.0, f, None);
    verify_value(f64::MIN, f, None);
    verify_value(f64::MAX, f, None);
    verify_value(0.0f64.next_up(), f, None);

    let mut r = Random::new();

    let mut len_sum = 0u64;
    let mut len_max = 0usize;
    for _i in 0..VERIFY_RANDOM_COUNT {
        let mut d;
        while {
            let u = (u64::from(r.get()) << 32) | u64::from(r.get());
            d = f64::from_bits(u);
            d.is_nan() || d.is_infinite()
        } {}
        let len = verify_value(d, f, None);
        len_sum += len as u64;
        len_max = usize::max(len_max, len);
    }

    let len_avg = len_sum as f64 / VERIFY_RANDOM_COUNT as f64;
    println!("OK. Length Avg = {len_avg:.3}, Max = {len_max}");
}

fn verify_all() {
    for test in TESTS {
        if test.fname != "null" {
            verify(test.dtoa, test.fname);
        }
    }
}

struct RandomDigitData;

impl RandomDigitData {
    const MAX_DIGIT: usize = 17;
    const COUNT: usize = 100_000;

    fn get_data(digit: usize) -> &'static [f64; Self::COUNT] {
        assert!((1..=17).contains(&digit));

        static SINGLETON: OnceLock<Vec<f64>> = OnceLock::new();

        let data = SINGLETON.get_or_init(|| {
            let mut data = Vec::with_capacity(Self::MAX_DIGIT * Self::COUNT);

            let mut r = Random::new();

            for digit in 1..=Self::MAX_DIGIT {
                for _i in 0..Self::COUNT {
                    let mut d;
                    while {
                        let u = (u64::from(r.get()) << 32) | u64::from(r.get());
                        d = f64::from_bits(u);
                        d.is_nan() || d.is_infinite()
                    } {}

                    // Convert to string with limited digits, and convert it back.
                    let buffer = format!("{:.prec$e}", d, prec = digit - 1);
                    let roundtrip = buffer.parse().unwrap();

                    data.push(roundtrip);
                }
            }

            data
        });

        &data.as_chunks().0[digit - 1]
    }
}

fn bench_random_digit(f: F, fname: &str) {
    print!("Benchmarking randomdigit {fname:20} ... ");

    let mut min_duration = f64::MAX;
    let mut max_duration = f64::MIN;
    let mut total_duration = 0.0;

    for digit in 1..=RandomDigitData::MAX_DIGIT {
        let data = RandomDigitData::get_data(digit);

        let mut duration = f64::MAX;
        for _trial in 0..TRIAL {
            let timer = Instant::now();

            for _iteration in 0..ITERATION_PER_DIGIT {
                for &i in data {
                    f(i, &mut |repr| {
                        hint::black_box(repr);
                    });
                }
            }

            duration = f64::min(duration, timer.elapsed().as_secs_f64() * 1000.0);
        }

        duration *= 1e6 / (ITERATION_PER_DIGIT * RandomDigitData::COUNT) as f64; // convert to nano second per operation
        min_duration = f64::min(min_duration, duration);
        max_duration = f64::max(max_duration, duration);
        total_duration += duration;
    }
    println!(
        "[{:8.3}ns, {:8.3}ns] {:8.3}ns",
        min_duration,
        max_duration,
        total_duration / RandomDigitData::MAX_DIGIT as f64,
    );
}

fn bench(f: F, fname: &str) {
    bench_random_digit(f, fname);
}

fn bench_all() {
    for test in TESTS {
        bench(test.dtoa, test.fname);
    }
}

fn main() {
    verify_all();
    bench_all();
}
