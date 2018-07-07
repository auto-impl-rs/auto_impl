#![feature(proc_macro)]

extern crate auto_impl;

use auto_impl::auto_impl;


#[auto_impl(Fn)]
trait Foo {
    fn foo<'a>(&self, x: &'a i32) -> f32;
    // fn foo2(&self, _s: String) -> bool {
    //     true
    // }
}

// #[auto_impl(Box)]
// trait MyTrait<'a, T>
//     where T: AsRef<str>
// {
//     type Type1;
//     type Type2;

//     fn execute1<'b>(&'a self, arg1: &'b T) -> Result<Self::Type1, String>;
//     fn execute2(&self) -> Self::Type2;
//     fn execute3(self) -> Self::Type1;
//     fn execute4() -> &'static str;
// }

fn do_foo<T: Foo>(x: T) {
    x.foo(&3);
}



fn main() {
    do_foo(|_: &i32| 0.0);
}
