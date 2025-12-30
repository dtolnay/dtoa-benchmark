use crate::traits;
use rand::SeedableRng as _;
use rand::distr::{Distribution, StandardUniform};
use rand::rngs::SmallRng;

pub struct Data {
    pub f32: [Vec<f32>; 9],
    pub f64: [Vec<f64>; 17],
}

impl Data {
    pub fn random(count: usize) -> Self {
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
