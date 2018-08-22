//! Shows the error message for the case the `#[auto_impl]` wasn't applied to
//! a valid trait (in this case a struct).
//!
//! To build the example, run:
//!
//! ```
//! $ cargo rustc --example fail_on_struct -- --cfg fail
//! ```
//!
//! To build it in nightly mode with better error message, run:
//!
//! ```
//! $ cargo rustc --example fail_on_struct --features nightly -- --cfg fail
//! ```

#[cfg(fail)]
mod fail {
    use self::auto_impl::auto_impl;


    #[auto_impl(&, Box)]
    struct Foo {
        x: u32,
    }
}


fn main() {}
