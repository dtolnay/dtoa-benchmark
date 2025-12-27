use anyhow::{Result, bail};
use std::env;

pub enum Type {
    F32,
    F64,
}

pub fn parse() -> Result<Vec<(&'static crate::Impl, Type)>> {
    let mut args = env::args_os();
    args.next().unwrap();

    let mut benchmark = Vec::new();
    'args: for arg in args {
        if let Some(arg) = arg.to_str() {
            let (lib, ty) = match arg.split_once(':') {
                Some((lib, ty)) => (lib, Some(ty)),
                None => (arg, None),
            };
            for imp in crate::IMPLS {
                if imp.name == lib {
                    match ty {
                        None => {
                            benchmark.push((imp, Type::F32));
                            benchmark.push((imp, Type::F64));
                            continue 'args;
                        }
                        Some("f32") => {
                            benchmark.push((imp, Type::F32));
                            continue 'args;
                        }
                        Some("f64") => {
                            benchmark.push((imp, Type::F64));
                            continue 'args;
                        }
                        Some(_) => {}
                    }
                }
            }
        }
        bail!("unsupported: {}", arg.display());
    }

    if benchmark.is_empty() {
        for imp in crate::IMPLS {
            benchmark.push((imp, Type::F32));
            benchmark.push((imp, Type::F64));
        }
    }

    Ok(benchmark)
}
