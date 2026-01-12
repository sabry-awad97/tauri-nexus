//! Test that min/max attributes require float values.

use zod_rs_macros::ZodSchema;

#[derive(ZodSchema)]
struct Product {
    #[zod(min = "not_a_number")]
    price: f64,
}

fn main() {}
