#![feature(proc_macro)]

extern crate auto_impl;

use auto_impl::auto_impl;


#[auto_impl(Rc)]
trait Foo<'a, X, Y = i32> {
    fn foo(&self);
}

// #[auto_impl(Box)]
// trait MyTrait<'a, T>
//     where T: AsRef<str>
// {
//     // type Type1;
//     // type Type2;

//     // fn execute1<'b>(&'a self, arg1: &'b T) -> Result<Self::Type1, String>;
//     // fn execute2(&self) -> Self::Type2;
//     // fn execute3(self) -> Self::Type1;
//     // fn execute4() -> &'static str;
// }



fn main() {}
