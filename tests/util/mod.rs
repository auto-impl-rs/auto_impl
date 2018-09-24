use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
};

use build_plan::BuildPlan;
use libtest_mimic::Test;



/// Obtains the path of the `auto_impl` artifact.
///
/// This is the biggest problem of this test script. This function uses Cargo's
/// build-plan feature which is not yet stable. It's surprisingly difficult to
/// get the path of the artifact. Another possibility would be to create small
/// temporary Cargo projects. But this is also not a good solution.
pub(crate) fn get_dep_path() -> PathBuf {
    // Obtain the build plan from `cargo build`. This JSON plan will tell us
    // several things, including the path of the output of `auto_impl` (usually
    // an .so file on Linux).
    let output = Command::new(env!("CARGO"))
        .args(&["build", "-Z", "unstable-options", "--build-plan"])
        .stderr(Stdio::inherit())
        .output()
        .expect("failed to run `cargo build`");

    if !output.status.success() {
        panic!("failed to run `cargo build`");
    }

    // Parse JSON.
    let plan = BuildPlan::from_cargo_output(&output.stdout)
        .expect("unexpected Cargo build-plan output");

    // Get the path of our library artifact.
    let mut outputs = plan.invocations.into_iter()
        .find(|inv| inv.package_name == "auto_impl")
        .expect("`auto_impl` invocation not found in build plan")
        .outputs;

    assert!(outputs.len() == 1);

    outputs.remove(0).into()
}

fn get_manifest_path() -> PathBuf {
    match env::var_os("CARGO_MANIFEST_DIR") {
        Some(dir) => dir.into(),
        None => {
            println!("CARGO_MANIFEST_DIR not set, falling back to current directory");
            env::current_dir().expect("invalid current dir").into()
        }
    }
}


/// Iterates through the given folder and collects all `.rs` files as
/// tests. The folder name also serves as "kind" of the tests.
pub(crate) fn collect_tests(dir: &str) -> Vec<Test<PathBuf>> {
    let test_dir = get_manifest_path()
        .join("tests")
        .join(dir);

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
                    kind: dir.into(),
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

/// Executes `rustc` to compile the given file.
///
/// The output is captured and returned. The `dep_path` is used to pass a
/// correct `--extern` flag.
pub(crate) fn run_rustc(file_path: &Path, dep_path: &Path) -> process::Output {
    // Build value for `--extern`
    let mut extern_value = OsString::from("auto_impl=");
    extern_value.push(dep_path);

    // Build value for `--out-dir`
    let out_dir = get_manifest_path()
        .join("target")
        .join("compile-test");

    if !out_dir.exists() {
        fs::create_dir(&out_dir).expect("unable to create folder in target/");
    }

    // Execute rustc
    Command::new("rustc")
        .arg(file_path)
        .args(&["--crate-type", "lib"])
        .args(&["-Z", "no-codegen"])
        .args(&["--emit", "metadata"])
        .arg("--edition=2018")
        .arg("--out-dir").arg(&out_dir)
        .arg("--extern").arg(&extern_value)
        .output()
        .expect("failed to run `rustc`")
}
