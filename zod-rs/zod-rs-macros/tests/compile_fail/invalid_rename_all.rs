//! Test that invalid rename_all values produce errors.

use zod_rs_macros::ZodSchema;

#[derive(ZodSchema)]
#[zod(rename_all = "invalid_case")]
struct User {
    name: String,
}

fn main() {}
