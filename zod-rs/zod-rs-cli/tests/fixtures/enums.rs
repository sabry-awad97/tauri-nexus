//! Test fixture: Various enum types.

use zod_rs::ZodSchema;

/// Simple unit enum
#[derive(ZodSchema)]
pub enum Status {
    Active,
    Inactive,
    Pending,
}

/// Enum with data variants
#[derive(ZodSchema)]
pub enum Message {
    Text(String),
    Number(i32),
    Pair(String, i32),
}

/// Enum with struct variants
#[derive(ZodSchema)]
pub enum Event {
    Click { x: i32, y: i32 },
    KeyPress { key: String, modifiers: Vec<String> },
    Scroll { delta_x: f64, delta_y: f64 },
}

/// Mixed enum
#[derive(ZodSchema)]
pub enum ApiResponse {
    Success,
    Error(String),
    Data { payload: String, code: u32 },
}
