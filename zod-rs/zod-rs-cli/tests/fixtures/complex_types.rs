//! Test fixture: Complex types with generics and nested structures.

use std::collections::HashMap;
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub zip_code: String,
    pub country: String,
}

#[derive(ZodSchema)]
pub struct Person {
    pub name: String,
    pub age: u8,
    pub address: Option<Address>,
    pub tags: Vec<String>,
}

#[derive(ZodSchema)]
pub struct Organization {
    pub name: String,
    pub members: Vec<Person>,
    pub metadata: HashMap<String, String>,
}
