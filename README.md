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
Then these fastest durations are averaged across the 17 f64 precision groups to
produce the table below.

Build and run the benchmark yourself using `cargo run --release`.

## Results

The following results are measured on a 2025 AMD Ryzen Threadripper 9975WX and
2024 Apple M4 Max, each using Rust 1.92.0:

<table>
  <tr><th rowspan="2" valign="bottom">Library</th><td align="center" colspan="2"><i>9975WX (x86_64)</i></td><td align="center" colspan="2"><i>M4 Max (aarch64)</i></td></tr>
  <tr><th>Time (ns)</th><th>Speedup</th><th>Time (ns)</th><th>Speedup</th></tr>
  <tr><td><a href="https://doc.rust-lang.org/std/fmt/trait.Display.html">libcore</a></td><td align="right">67.7</td><td align="right">1.00&times;</td><td align="right">61.1</td><td align="right">1.00&times;</td></tr>
  <tr><td><a href="https://github.com/dtolnay/dtoa">dtoa</a></td><td align="right">41.8</td><td align="right">1.62&times;</td><td align="right">43.7</td><td align="right">1.40&times;</td></tr>
  <tr><td><a href="https://github.com/dtolnay/ryu">ryu</a></td><td align="right">31.2</td><td align="right">2.17&times;</td><td align="right">26.4</td><td align="right">2.31&times;</td></tr>
  <tr><td><a href="https://github.com/Alexhuszagh/rust-lexical">lexical</a></td><td align="right">24.0</td><td align="right">2.82&times;</td><td align="right">21.2</td><td align="right">2.88&times;</td></tr>
  <tr><td><a href="https://github.com/andrepd/teju-jagua-rs">teju</a></td><td align="right">23.0</td><td align="right">2.94&times;</td><td align="right">19.0</td><td align="right">3.22&times;</td></tr>
  <tr><td><a href="https://github.com/dtolnay/zmij">zmij</a></td><td align="right">11.0</td><td align="right">6.15&times;</td><td align="right">7.7</td><td align="right">7.93&times;</td></tr>
</table>

![performance](https://raw.githubusercontent.com/dtolnay/dtoa-benchmark/master/performance.png)
