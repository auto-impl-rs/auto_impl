//! Test script to make sure some files can't be compiled.
//!
//! This file only contains the logic of compile-fail tests. The actual tests
//! are in the directory `compile-fail/`.

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
    let tests = util::collect_tests("compile-fail");
    run_tests(&args, tests, |test| {
        let output = util::run_rustc(&test.data, &dep_path);

        if output.status.success() {
            Outcome::Failed {
                msg: Some("Expected compiler error, but file got compiled without error!".into()),
            }
        } else {
            Outcome::Passed
        }
    }).exit();
}
