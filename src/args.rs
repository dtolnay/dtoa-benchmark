use anyhow::{Result, bail};
use std::env;

pub struct Args {
    pub benchmark: Vec<(&'static str, Type)>,
    pub unpredictable: bool,
}

pub enum Type {
    F32(crate::F<f32>),
    F64(crate::F<f64>),
}

pub fn parse() -> Result<Args> {
    let mut args = env::args_os();
    args.next().unwrap();

    let mut benchmark = Vec::new();
    let mut unpredictable = false;
    'args: for arg in args {
        if let Some(arg) = arg.to_str() {
            if arg == "--unpredictable" {
                unpredictable = true;
                continue;
            }
            let (lib, ty) = match arg.split_once(':') {
                Some((lib, ty)) => (lib, Some(ty)),
                None => (arg, None),
            };
            for imp in crate::IMPLS {
                if imp.name == lib {
                    match ty {
                        None => {
                            if let Some(f) = imp.f32 {
                                benchmark.push((imp.name, Type::F32(f)));
                            }
                            if let Some(f) = imp.f64 {
                                benchmark.push((imp.name, Type::F64(f)));
                            }
                            continue 'args;
                        }
                        Some("f32") => {
                            if let Some(f) = imp.f32 {
                                benchmark.push((imp.name, Type::F32(f)));
                                continue 'args;
                            }
                        }
                        Some("f64") => {
                            if let Some(f) = imp.f64 {
                                benchmark.push((imp.name, Type::F64(f)));
                                continue 'args;
                            }
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
            if let Some(f) = imp.f32 {
                benchmark.push((imp.name, Type::F32(f)));
            }
            if let Some(f) = imp.f64 {
                benchmark.push((imp.name, Type::F64(f)));
            }
        }
    }

    Ok(Args {
        benchmark,
        unpredictable,
    })
}
