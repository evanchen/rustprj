extern crate rand;

use rand::Rng;

pub fn default_random_value(literal: &str) -> String {
    let mut rng = rand::thread_rng();
    let str = match literal {
        "i8" => {
            format!("{}", rng.gen::<i32>() as i8)
        }
        "u8" => {
            format!("{}", rng.gen::<i32>() as u8)
        }
        "i16" => {
            format!("{}", rng.gen::<i32>() as i16)
        }
        "u16" => {
            format!("{}", rng.gen::<i32>() as u16)
        }
        "i32" => {
            format!("{}", rng.gen::<i32>())
        }
        "u32" => {
            format!("{}", rng.gen::<u32>())
        }
        "i64" => {
            format!("{}", rng.gen::<i64>())
        }
        "u64" => {
            format!("{}", rng.gen::<u64>())
        }
        "bool" => {
            format!("{}", rng.gen::<bool>())
        }
        "f32" => {
            format!("{}", rng.gen::<f32>())
        }
        "f64" => {
            format!("{}", rng.gen::<f64>())
        }
        "String" => {
            let times = rng.gen_range(1..(10 * (2 ^ 20) / 4));
            "test".repeat(times)
        }
        _ => String::new(),
    };
    str
}
