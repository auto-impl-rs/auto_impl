#![feature(proc_macro)]

extern crate auto_impl;

use auto_impl::auto_impl;


#[auto_impl(&, Box, &mut)]
trait Foo<'a, X, Y = i32> {
    // fn foo(&self);
}



fn main() {}
