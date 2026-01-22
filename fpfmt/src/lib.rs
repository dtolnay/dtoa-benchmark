#![allow(mixed_script_confusables)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::must_use_candidate,
    clippy::unreadable_literal
)]

use std::fmt::{self, Display};
use std::ops::BitOr;

mod pow10;

// bool2 converts b to an integer: 1 for true, 0 for false.
fn bool2<T>(b: bool) -> T
where
    T: From<bool>,
{
    T::from(b)
}

// unpack64 returns m, e such that f = m * 2**e.
// The caller is expected to have handled 0, NaN, and ±Inf already.
fn unpack64(f: f64) -> (u64, isize) {
    const SHIFT: isize = 64 - 53;
    const MIN_EXP: isize = -(1074 + SHIFT);
    let b = f.to_bits();
    let mut m = (1 << 63) | ((b & ((1 << 52) - 1)) << SHIFT);
    let mut e = ((b >> 52) & ((1 << SHIFT) - 1)) as isize;
    if e == 0 {
        m &= !(1 << 63);
        e = MIN_EXP;
        let s = m.leading_zeros();
        return (m << s, e - s as isize);
    }
    (m, (e - 1) + MIN_EXP)
}

// An unrounded represents an unrounded value.
#[derive(Copy, Clone)]
struct Unrounded(u64);

impl From<bool> for Unrounded {
    fn from(b: bool) -> Self {
        Unrounded(u64::from(b))
    }
}

impl BitOr for Unrounded {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Unrounded(self.0 | rhs.0)
    }
}

impl Display for Unrounded {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "⟨{}.{}{}⟩",
            self.0 >> 2,
            5 * ((self.0 >> 1) & 1),
            &"+"[(1 - (self.0 & 1)) as usize..],
        )
    }
}

impl Unrounded {
    fn floor(self) -> u64 {
        self.0 >> 2
    }

    #[expect(dead_code)]
    fn round_half_down(self) -> u64 {
        (self.0 + 1) >> 2
    }

    fn round(self) -> u64 {
        (self.0 + 1 + ((self.0 >> 2) & 1)) >> 2
    }

    #[expect(dead_code)]
    fn round_half_up(self) -> u64 {
        (self.0 + 2) >> 2
    }

    fn ceil(self) -> u64 {
        (self.0 + 3) >> 2
    }

    fn nudge(self, δ: isize) -> Unrounded {
        Unrounded(self.0.wrapping_add(δ as u64))
    }

    #[expect(dead_code)]
    fn div(self, d: u64) -> Unrounded {
        let x = self.0;
        Unrounded(x / d) | Unrounded(self.0 & 1) | bool2::<Unrounded>(!x.is_multiple_of(d))
    }

    #[expect(dead_code)]
    fn rsh(self, s: isize) -> Unrounded {
        Unrounded(self.0 >> s)
            | Unrounded(self.0 & 1)
            | bool2::<Unrounded>(self.0 & ((1 << s) - 1) != 0)
    }
}

// log10_pow2(x) returns ⌊log₁₀ 2**x⌋ = ⌊x * log₁₀ 2⌋.
fn log10_pow2(x: isize) -> isize {
    // log₁₀ 2 ≈ 0.30102999566 ≈ 78913 / 2^18
    (x * 78913) >> 18
}

// log2_pow10(x) returns ⌊log₂ 10**x⌋ = ⌊x * log₂ 10⌋.
fn log2_pow10(x: isize) -> isize {
    // log₂ 10 ≈ 3.32192809489 ≈ 108853 / 2^15
    (x * 108853) >> 15
}

// uint64pow10[x] is 10**x.
static U64_POW10: [u64; 20] = {
    let mut u64_pow10 = [1u64; 20];
    let mut x = 1;
    while x < u64_pow10.len() {
        u64_pow10[x] = 10 * u64_pow10[x - 1];
        x += 1;
    }
    u64_pow10
};

// Short computes the shortest formatting of f,
// using as few digits as possible that will still round trip
// back to the original f64.
fn short(f: f64) -> (u64, isize) {
    const MIN_EXP: isize = -1085;

    let (m, e) = unpack64(f);

    let p;
    let min: u64;
    let mut z = 11; // extra zero bits at bottom of m; 11 for 53-bit m
    if m == 1 << 63 && e > MIN_EXP {
        p = -skewed(e + z);
        min = m - (1 << (z - 2)); // min = m - 1/4 * 2**(e+z)
    } else {
        if e < MIN_EXP {
            z = 11 + (MIN_EXP - e);
        }
        p = -log10_pow2(e + z);
        min = m - (1 << (z - 1)); // min = m - 1/2 * 2**(e+z)
    }
    let max = m + (1 << (z - 1)); // max = m + 1/2 * 2**(e+z)
    let odd = (m >> z) as isize & 1;

    let pre = prescale(e, p, log2_pow10(p));
    let dmin = uscale(min, pre).nudge(odd).ceil();
    let dmax = uscale(max, pre).nudge(-odd).floor();

    let mut d = dmax / 10;
    if d * 10 >= dmin {
        return trim_zeros(d, -(p - 1));
    }
    d = dmin;
    if d < dmax {
        d = uscale(m, pre).round();
    }
    (d, -p)
}

// skewed computes the skewed footprint of m * 2**e,
// which is ⌊log₁₀ 3/4 * 2**e⌋ = ⌊e*(log₁₀ 2)-(log₁₀ 4/3)⌋.
fn skewed(e: isize) -> isize {
    (e * 631305 - 261663) >> 21
}

// trimZeros removes trailing zeros from x * 10**p.
// If x ends in k zeros, trimZeros returns x/10**k, p+k.
// It assumes that x ends in at most 16 zeros.
fn trim_zeros(mut x: u64, mut p: isize) -> (u64, isize) {
    const INV5P8: u64 = 0xc767074b22e90e21; // inverse of 5**8
    const INV5P4: u64 = 0xd288ce703afb7e91; // inverse of 5**4
    const INV5P2: u64 = 0x8f5c28f5c28f5c29; // inverse of 5**2
    const INV5: u64 = 0xcccccccccccccccd; // inverse of 5

    // Cut 1 zero, or else return.
    let d = x.wrapping_mul(INV5).rotate_right(1);
    if d <= u64::MAX / 10 {
        x = d;
        p += 1;
    } else {
        return (x, p);
    }

    // Cut 8 zeros, then 4, then 2, then 1.
    let d = x.wrapping_mul(INV5P8).rotate_right(8);
    if d <= u64::MAX / 100000000 {
        x = d;
        p += 8;
    }
    let d = x.wrapping_mul(INV5P4).rotate_right(4);
    if d <= u64::MAX / 10000 {
        x = d;
        p += 4;
    }
    let d = x.wrapping_mul(INV5P2).rotate_right(2);
    if d <= u64::MAX / 100 {
        x = d;
        p += 2;
    }
    let d = x.wrapping_mul(INV5).rotate_right(1);
    if d <= u64::MAX / 10 {
        x = d;
        p += 1;
    }
    (x, p)
}

// A pmHiLo represents hi<<64 - lo.
#[derive(Copy, Clone)]
struct PmHiLo {
    hi: u64,
    lo: u64,
}

// A scaler holds derived scaling constants for a given e, p pair.
#[derive(Copy, Clone)]
struct Scaler {
    pm: PmHiLo,
    s: isize,
}

// prescale returns the scaling constants for e, p.
// lp must be log2Pow10(p).
fn prescale(e: isize, p: isize, lp: isize) -> Scaler {
    let (hi, lo) = pow10::TAB[(p - pow10::MIN) as usize];
    Scaler {
        pm: PmHiLo { hi, lo },
        s: -(e + lp + 3),
    }
}

// uscale returns unround(x * 2**e * 10**p).
// The caller should pass c = prescale(e, p, log2Pow10(p))
// and should have left-justified x so its high bit is set.
fn uscale(x: u64, c: Scaler) -> Unrounded {
    let (mut hi, mid) = mul64(x, c.pm.hi);
    let mut sticky = 1u64;
    if (hi & ((1 << (c.s & 63)) - 1)) == 0 {
        let (mid2, _) = mul64(x, c.pm.lo);
        sticky = bool2::<u64>(mid.wrapping_sub(mid2) > 1);
        hi -= bool2::<u64>(mid < mid2);
    }
    Unrounded((hi >> c.s) | sticky)
}

// Go's bits.Mul64
// Rust's u64::widening_mul but reverse order in the return tuple
fn mul64(x: u64, y: u64) -> (u64, u64) {
    let product = u128::from(x) * u128::from(y);
    ((product >> 64) as u64, product as u64)
}

// Fmt formats d, p into s in exponential notation.
// The caller must pass nd set to the number of digits in d.
// It returns the number of bytes written to s.
fn fmt(s: &mut [u8], d: u64, mut p: isize, nd: usize) -> usize {
    // Put digits into s, leaving room for decimal point.
    format_base10(&mut s[1..=nd], d);
    p += (nd - 1) as isize;

    // Move first digit up and insert decimal point.
    s[0] = s[1];
    let mut n = nd;
    if n > 1 {
        s[1] = b'.';
        n += 1;
    }

    // Add 2- or 3-digit exponent.
    s[n] = b'e';
    if p < 0 {
        s[n + 1] = b'-';
        p = -p;
    } else {
        s[n + 1] = b'+';
    }
    if p < 100 {
        s[n + 2] = I2A[(p * 2) as usize];
        s[n + 3] = I2A[(p * 2 + 1) as usize];
        return n + 4;
    }
    s[n + 2] = b'0' + (p / 100) as u8;
    s[n + 3] = I2A[((p % 100) * 2) as usize];
    s[n + 4] = I2A[((p % 100) * 2 + 1) as usize];
    n + 5
}

// Digits returns the number of decimal digits in d.
fn digits(d: u64) -> usize {
    let nd = log10_pow2(64 - d.leading_zeros() as isize);
    nd as usize + bool2::<usize>(d >= U64_POW10[nd as usize])
}

// i2a is the formatting of 00..99 concatenated,
// a lookup table for formatting [0, 99].
const I2A: [u8; 200] = *b"\
    00010203040506070809\
    10111213141516171819\
    20212223242526272829\
    30313233343536373839\
    40414243444546474849\
    50515253545556575859\
    60616263646566676869\
    70717273747576777879\
    80818283848586878889\
    90919293949596979899";

// formatBase10 formats the decimal representation of u into a.
// The caller is responsible for ensuring that a is big enough to hold u.
// If a is too big, leading zeros will be filled in as needed.
fn format_base10(a: &mut [u8], mut u: u64) {
    let mut nd = a.len();
    while nd >= 8 {
        // Format last 8 digits (4 pairs).
        let x3210 = (u % 100_000_000) as u32;
        u /= 100_000_000;
        let (x32, x10) = (x3210 / 10000, x3210 % 10000);
        let (x1, x0) = ((x10 / 100) * 2, (x10 % 100) * 2);
        let (x3, x2) = ((x32 / 100) * 2, (x32 % 100) * 2);
        a[nd - 1] = I2A[x0 as usize + 1];
        a[nd - 2] = I2A[x0 as usize];
        a[nd - 3] = I2A[x1 as usize + 1];
        a[nd - 4] = I2A[x1 as usize];
        a[nd - 5] = I2A[x2 as usize + 1];
        a[nd - 6] = I2A[x2 as usize];
        a[nd - 7] = I2A[x3 as usize + 1];
        a[nd - 8] = I2A[x3 as usize];
        nd -= 8;
    }

    let mut x = u as u32;
    if nd >= 4 {
        // Format last 4 digits (2 pairs).
        let x10 = x % 10000;
        x /= 10000;
        let (x1, x0) = ((x10 / 100) * 2, (x10 % 100) * 2);
        a[nd - 1] = I2A[x0 as usize + 1];
        a[nd - 2] = I2A[x0 as usize];
        a[nd - 3] = I2A[x1 as usize + 1];
        a[nd - 4] = I2A[x1 as usize];
        nd -= 4;
    }
    if nd >= 2 {
        // Format last 2 digits.
        let x0 = (x % 100) * 2;
        x /= 100;
        a[nd - 1] = I2A[x0 as usize + 1];
        a[nd - 2] = I2A[x0 as usize];
        nd -= 2;
    }
    if nd > 0 {
        // Format final digit.
        a[0] = b'0' + x as u8;
    }
}

#[derive(Default)]
pub struct Buffer {
    bytes: [u8; 24],
}

impl Buffer {
    pub fn new() -> Self {
        Buffer::default()
    }

    pub fn format_finite(&mut self, f: f64) -> &str {
        self.bytes[0] = b'-';
        let begin = usize::from(f.is_sign_negative());
        let len = if f == 0.0 {
            self.bytes[begin..begin + 3].copy_from_slice(b"0.0");
            3
        } else {
            let (d, p) = short(f);
            fmt(&mut self.bytes[begin..], d, p, digits(d))
        };
        unsafe { str::from_utf8_unchecked(&self.bytes[..begin + len]) }
    }
}
