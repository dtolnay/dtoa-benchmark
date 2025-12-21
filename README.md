# dtoa Benchmark

This is a Rust port of {fmt}'s [dtoa-benchmark][fmtlib] by Victor Zverovich,
which is a fork of Milo Yip's [dtoa-benchmark][miloyip].

[fmtlib]: https://github.com/fmtlib/dtoa-benchmark
[miloyip]: https://github.com/miloyip/dtoa-benchmark

This benchmark evaluates the performance of conversion from double precision
IEEE-754 floating point (f64) to ASCII string.

## Procedure

**Input data:** The benchmark generates f64 bit patterns using a simplistic
pseudorandom number generator, bitcasting from 64-bit integer to f64 and
discarding +/-inf and NaN. It truncates each value to a limited precision
ranging from 1 to 17 decimal digits in the significand, producing an equal
number of values of each precision.

**Measurement:** For each dtoa library, for each precision group, we perform
multiple passes over the input data and take the duration of the fastest pass.
Then these fastest durations are averaged across the 17 precision groups to
produce the table below.

Build and run the benchmark yourself using `cargo run --release`.

## Results

The following results are measured on 2018 AMD Ryzen Threadripper 2990WX using
Rust 1.92.0.

| Function  | Time (ns) | Speedup     |
|-----------|----------:|------------:|
| [libcore] | 119.5     | 1.00&times; |
| [dtoa]    | 68.3      | 1.75&times; |
| [ryu]     | 48.1      | 2.48&times; |
| [teju]    | 35.1      | 3.40&times; |
| [zmij]    | 24.5      | 4.88&times; |

[libcore]: https://doc.rust-lang.org/std/fmt/trait.Display.html
[dtoa]: https://github.com/dtolnay/dtoa
[ryu]: https://github.com/dtolnay/ryu
[teju]: https://github.com/andrepd/teju-jagua-rs
[zmij]: https://github.com/dtolnay/zmij
