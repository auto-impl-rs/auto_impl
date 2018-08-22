//! Shows the error message for the case that `#[auto_impl]` was used with
//! incorrect proxy types. Only proxy types like `&` and `Box` are allowed.
//!
//! To build the example, run:
//!
//! ```
//! $ cargo rustc --example fail_invalid_proxy -- --cfg fail
//! ```
//!
//! To build it in nightly mode with better error message, run:
//!
//! ```
//! $ cargo rustc --example fail_invalid_proxy --features nightly -- --cfg fail
//! ```

#[cfg(fail)]
mod fail {
    use self::auto_impl::auto_impl;


    #[auto_impl(Boxxi)]
    trait Foo {
        fn foo(&self) -> u32;
    }
}


fn main() {}
