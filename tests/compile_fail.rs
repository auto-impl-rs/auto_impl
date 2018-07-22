//! Test script to make sure some files can't be compiled
//!
//! This file only contains the logic of compile-fail tests. The actual tests
//! are in the directory `compile-fail`.

extern crate libtest_mimic;
extern crate build_plan;

use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use libtest_mimic::{run_tests, Arguments, Test, Outcome};


fn main() {
    // Parse CLI args
    let args = Arguments::from_args();

    // Get the path of the `auto_impl` manifest
    let dep_path = get_dep_path();

    // Run all tests and exit the application appropriately
    let tests = collect_tests();
    run_tests(&args, tests, |test| run_test(test, &dep_path))
        .exit();
}

/// Iterates through the `compile-fail` folder and collects all `.rs` files as
/// tests.
fn collect_tests() -> Vec<Test<PathBuf>> {
    // Get current path
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");

    let test_dir = Path::new(&manifest_dir)
        .join("tests")
        .join("compile-fail");

    // Collect all sub-directories in the current directory
    fs::read_dir(test_dir)
        .expect("failed to list directory contents")
        .filter_map(|entry| {
            let entry = entry.expect("failed to read directory entry");
            let file_type = entry
                .file_type()
                .expect("failed to determine entry type");

            // If this entry is a file with the extension `.rs`, we treat it as
            // test.
            let path = entry.path();
            if file_type.is_file() && path.extension() == Some(OsStr::new("rs")) {
                let name = path.file_stem().unwrap().to_string_lossy().into_owned();
                Some(Test {
                    name,
                    kind: "compile-fail".into(),
                    is_ignored: false,
                    is_bench: false,
                    data: path,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Runs the the given test.
fn run_test(test: &Test<PathBuf>, dep_path: &Path) -> Outcome {
    let path = &test.data;

    // Execute `rustc` and capture its outputs
    let mut extern_value = OsString::from("auto_impl=");
    extern_value.push(dep_path);
    let output = Command::new("rustc")
        .arg(&path)
        .args(&["--crate-type", "lib"])
        .args(&["-Z", "no-codegen"])
        .arg("--extern")
        .arg(&extern_value)
        .output()
        .expect("failed to run `rustc`");

    // TODO: check stderr/stdout if requested

    if output.status.success() {
        Outcome::Failed {
            msg: Some("Expected compiler error, but file got compiled without error!".into())
        }
    } else {
        Outcome::Passed
    }
}

/// Obtains the path of the `auto_impl` artifact.
///
/// This is the biggest problem of this test script. This function uses Cargo's
/// build-plan feature which is not yet stable. It's surprisingly difficult to
/// get the path of the artifact. Another possibility would be to create small
/// temporary Cargo projects. But this is also not a good solution.
fn get_dep_path() -> PathBuf {
    // Obtain the build plan from `cargo build`. This JSON plan will tell us
    // several things, including the path of the output of `auto_impl` (usually
    // an .so file on Linux).
    let output = Command::new(env!("CARGO"))
        .args(&["build", "-Z", "unstable-options", "--build-plan"])
        .output()
        .expect("failed to run `cargo build`");

    // Parse JSON.
    let plan = build_plan::BuildPlan::from_cargo_output(&output.stdout)
        .expect("unexpected Cargo build-plan output");

    // Get the path of our library artifact.
    let mut outputs = plan.invocations.into_iter()
        .find(|inv| inv.package_name == "auto_impl")
        .expect("`auto_impl` invocation not found in build plan")
        .outputs;

    assert!(outputs.len() == 1);

    outputs.remove(0).into()
}
