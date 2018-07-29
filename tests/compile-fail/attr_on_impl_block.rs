#![feature(use_extern_macros)]

extern crate auto_impl;
use auto_impl::auto_impl;


trait Foo {}

#[auto_impl(&, &mut)]
impl Foo for usize {}
