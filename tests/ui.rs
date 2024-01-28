use trybuild::TestCases;

#[test]
fn ui_compile_pass() {
    let t = TestCases::new();
    t.pass("tests/compile-pass/*.rs");
}

#[rustversion::nightly]
#[test]
fn ui_compile_fail() {
    let t = TestCases::new();
    t.compile_fail("tests/compile-fail/*.rs");
}

#[rustversion::since(1.51)]
#[test]
fn ui_recent_compile_pass() {
    let t = TestCases::new();
    t.pass("tests/recent/compile-pass/*.rs");
}
