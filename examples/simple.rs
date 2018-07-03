#![feature(proc_macro)]

extern crate auto_impl;

use auto_impl::auto_impl;


#[auto_impl(&, Box, &mut)]
trait Foo {
    fn foo(&self);
}



fn main() {}
