//! Test fixture: Simple struct with basic types.

use zod_rs::ZodSchema;

#[derive(ZodSchema)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub active: bool,
}

#[derive(ZodSchema)]
pub struct Post {
    pub id: u64,
    pub title: String,
    pub content: String,
    pub author_id: u32,
}
