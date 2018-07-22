extern crate libtest_mimic;
extern crate build_plan;

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use libtest_mimic::{run_tests, Arguments, Test, Outcome};

fn main() {
    let args = Arguments::from_args();

    let dep_path = get_dep_path();

    let tests = collect_tests();
    run_tests(&args, tests, |test| run_test(test, &dep_path))
        .exit();
}

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

            if file_type.is_dir() {
                let path = entry.path();
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
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

/// Run the single test in the given directory.
fn run_test(test: &Test<PathBuf>, dep_path: &Path) -> Outcome {
    // Find .rs files and determine which file to use.
    let dir = &test.data;
    let main_path = dir.join("main.rs");
    let lib_path = dir.join("lib.rs");

    let main_exists = main_path.exists() && main_path.is_file();
    let lib_exists = lib_path.exists() && lib_path.is_file();

    let (path, crate_type) = match (main_exists, lib_exists) {
        (false, false) => {
            panic!("No 'main.rs' or 'lib.rs' file found in '{}'", dir.display());
        }
        (true, true) => {
            panic!(
                "'main.rs' AND 'lib.rs' file found in '{}' (only one is allowed!)",
                dir.display()
            );
        }
        (true, false) => (main_path, "bin"),
        (false, true) => (lib_path, "lib"),
    };


    // Execute `rustc` and capture its outputs
    let mut extern_value = OsString::from("auto_impl=");
    extern_value.push(dep_path);
    let output = Command::new("rustc")
        .arg(&path)
        .args(&["--crate-type", crate_type])
        .args(&["-Z", "no-trans"])
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

fn get_dep_path() -> PathBuf {
    // Obtain the build plan from `cargo build`. This JSON plan will tell us
    // several things, including the path of the output of `auto_impl` (usually
    // an .so file on Linux).
    let output = Command::new("cargo")
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
