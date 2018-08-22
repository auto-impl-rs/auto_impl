
extern crate auto_impl;
use auto_impl::auto_impl;


#[auto_impl(Fn)]
trait Foo {
    const LEN: usize;

    fn a(&self);
}
