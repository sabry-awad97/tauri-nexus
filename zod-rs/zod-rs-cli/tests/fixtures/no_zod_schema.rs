//! Test fixture: Types without ZodSchema derive (should be ignored).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotAZodType {
    pub id: u32,
    pub name: String,
}

#[derive(Debug)]
pub enum PlainEnum {
    A,
    B,
    C,
}

pub struct PlainStruct {
    pub value: i32,
}
