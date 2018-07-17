/*!
Try running `cargo expand` on this crate to see the output of `#[auto_impl]`.
*/

#![feature(use_extern_macros)]

extern crate auto_impl;

use auto_impl::auto_impl;

#[auto_impl(Fn)]
trait FnTrait1 {
    fn execute(&self);
}

#[auto_impl(Fn)]
trait FnTrait2<'a, T> {
    fn execute<'b, 'c>(&'a self, arg1: &'b T, arg2: &'c T, arg3: &'static str) -> Result<T, String>;
}

#[auto_impl(FnMut)]
trait FnTrait3 {
    fn execute(&mut self);
}

#[auto_impl(Arc, Box, Rc, &, &mut)]
trait RefTrait1<'a, T: for<'b> Into<&'b str>> {
    type Type1;
    type Type2;

    fn execute1<'b>(&'a self, arg1: &'b T) -> Result<Self::Type1, String>;
    fn execute2(&self) -> Self::Type2;
}

#[auto_impl(Box)]
trait BoxTrait1<'a, T: for<'b> Into<&'b str>> {
    type Type1;
    type Type2;

    fn execute1<'b>(&'a self, arg1: &'b T) -> Result<Self::Type1, String>;
    fn execute2(&mut self, arg1: i32) -> Self::Type2;
    fn execute3(self) -> Self::Type1;
    fn execute4(arg1: String) -> Result<i32, String>;
}

fn main() {
    println!("Hello, world!");
}
