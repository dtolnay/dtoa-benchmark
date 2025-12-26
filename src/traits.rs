use std::fmt::{Debug, LowerExp};
use std::str::FromStr;

pub trait Float: Copy + LowerExp + FromStr<Err: Debug> {
    type Bits;
    fn from_bits(bits: Self::Bits) -> Self;
    fn is_finite(self) -> bool;
}

impl Float for f32 {
    type Bits = u32;
    fn from_bits(bits: u32) -> Self {
        f32::from_bits(bits)
    }
    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }
}

impl Float for f64 {
    type Bits = u64;
    fn from_bits(bits: u64) -> Self {
        f64::from_bits(bits)
    }
    fn is_finite(self) -> bool {
        f64::is_finite(self)
    }
}
