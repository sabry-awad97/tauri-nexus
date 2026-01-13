//! Test fixture: Mixed types - some with ZodSchema, some without.

use zod_rs::ZodSchema;

/// This type should be included
#[derive(ZodSchema)]
pub struct IncludedType {
    pub id: u32,
    pub name: String,
}

/// This type should be excluded (no ZodSchema)
#[derive(Debug, Clone)]
pub struct ExcludedType {
    pub value: i32,
}

/// This type should be included
#[derive(ZodSchema)]
pub enum IncludedEnum {
    A,
    B,
}

/// This type should be excluded (no ZodSchema)
pub enum ExcludedEnum {
    X,
    Y,
}

/// Another included type
#[derive(ZodSchema)]
pub struct AnotherIncluded {
    pub data: Vec<String>,
    pub optional_field: Option<i64>,
}
