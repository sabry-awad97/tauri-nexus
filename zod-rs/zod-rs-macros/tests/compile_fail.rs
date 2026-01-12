//! Compile-fail tests using trybuild.
//!
//! These tests verify that the macro produces appropriate compile errors
//! for invalid inputs.

#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
