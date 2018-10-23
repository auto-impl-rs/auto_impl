//! Example to demonstrate how `auto_impl` chooses a name for the type
//! parameter.
//!
//! For documentation and compiler errors it would be nice to have very simple
//! names for the type parameter:
//!
//! ```rust
//! // not nice
//! impl<AutoImplT> Foo for &AutoImplT { ...}
//!
//! // better
//! impl<T> Foo for &T { ... }
//! ```
//!
//! `auto_impl` tries the full alphabet, picking a name that is not yet taken.
//! "Not taken" means that the name is not used anywhere in the `impl` block.
//! Right now, we are a bit careful and mark all names as "taken" that are used
//! in the trait def -- apart from names only appearing in the body of provided
//! methods.
//!
//! In the trait below, a bunch of type names are already "taken": T--Z and
//! A--H. Thus, the name `I` will be used.
//!
//! Thus, the name `H` is used. Run `cargo expand --example names` to see the
//! output.


// This code is really ugly on purpose...
#![allow(non_snake_case, dead_code, unused_variables)]

use auto_impl::auto_impl;



struct X {}
trait Z {}

struct C {}
struct E<T>(Vec<T>);
struct F {}

struct G<T>(Vec<T>);
struct H {}

#[auto_impl(&)]
trait U<'a, T, V> {
    const W: Option<Box<&'static X>>;

    type Y: Z;

    fn A(&self, B: C, D: E<&[F; 1]>) -> G<fn((H,))> {
        let H = ();
        unimplemented!()
    }
}

fn main() {}
