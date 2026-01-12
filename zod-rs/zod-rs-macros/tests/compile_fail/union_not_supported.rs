//! Test that unions are not supported.

use zod_rs_macros::ZodSchema;

#[derive(ZodSchema)]
union MyUnion {
    i: i32,
    f: f32,
}

fn main() {}
