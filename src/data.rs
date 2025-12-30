use crate::traits;
use rand::SeedableRng as _;
use rand::distr::{Distribution, StandardUniform};
use rand::rngs::SmallRng;
use rand::seq::SliceRandom as _;

pub struct Data {
    pub f32: DataForType<f32, 9>,
    pub f64: DataForType<f64, 17>,
}

pub struct DataForType<T, const N: usize> {
    pub count: usize,
    pub mixed: Vec<T>,
    pub by_precision: [Vec<T>; N],
    pub unpredictable: bool,
}

impl Data {
    pub fn random(count: usize, unpredictable: bool) -> Self {
        let mut rng = SmallRng::seed_from_u64(1);
        Data {
            f32: DataForType::random(&mut rng, count, unpredictable),
            f64: DataForType::random(&mut rng, count, unpredictable),
        }
    }
}

impl<T, const N: usize> DataForType<T, N>
where
    T: traits::Float,
    StandardUniform: Distribution<T::Bits>,
{
    fn random(rng: &mut SmallRng, count: usize, unpredictable: bool) -> Self {
        let mut mixed = Vec::new();
        let mut by_precision = [const { Vec::new() }; N];
        if unpredictable {
            mixed.reserve_exact(count);
            for i in 0..count {
                mixed.push(sample(rng, i % N));
            }
            mixed.shuffle(rng);
            for (prec, vec) in by_precision.iter_mut().enumerate() {
                vec.reserve_exact(count * 2);
                vec.extend_from_slice(&mixed);
                for _ in 0..count {
                    vec.push(sample(rng, prec));
                }
                vec.shuffle(rng);
            }
        } else {
            for (prec, vec) in by_precision.iter_mut().enumerate() {
                vec.reserve_exact(count);
                for _i in 0..count {
                    vec.push(sample(rng, prec));
                }
            }
        }
        DataForType {
            count,
            mixed,
            by_precision,
            unpredictable,
        }
    }
}

fn sample<T>(rng: &mut SmallRng, prec: usize) -> T
where
    T: traits::Float,
    StandardUniform: Distribution<T::Bits>,
{
    loop {
        let bits = StandardUniform.sample(rng);
        let float = T::from_bits(bits);
        if float.is_finite() {
            // Convert to string with limited digits, and convert it back.
            return format!("{float:.prec$e}").parse().unwrap();
        }
    }
}
