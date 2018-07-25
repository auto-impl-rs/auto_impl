#![feature(use_extern_macros)]

extern crate auto_impl;

use auto_impl::auto_impl;


#[auto_impl(Rc)]
trait Foo {
    fn foo(&mut self);
}
