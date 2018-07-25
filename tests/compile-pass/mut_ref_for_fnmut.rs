#![feature(use_extern_macros)]

extern crate auto_impl;
use auto_impl::auto_impl;


#[auto_impl(FnMut)]
trait FnTrait3 {
    fn execute(&mut self);
}
