//! Test script to make sure some files can be compiled.
//!
//! This file only contains the logic of compile-pass tests. The actual tests
//! are in the directory `compile-pass/`.

extern crate libtest_mimic;
extern crate build_plan;

use libtest_mimic::{run_tests, Arguments, Outcome};

mod util;


fn main() {
    // Parse CLI args
    let args = Arguments::from_args();

    // Get the path of the `auto_impl` manifest
    let dep_path = util::get_dep_path();

    // Run all tests and exit the application appropriately
    let tests = util::collect_tests("compile-pass");
    run_tests(&args, tests, |test| {
        let output = util::run_rustc(&test.data, &dep_path);

        if output.status.success() {
            Outcome::Passed
        } else {
            Outcome::Failed {
                msg: Some(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        }
    }).exit();
}
