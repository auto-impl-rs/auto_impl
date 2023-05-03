use auto_impl::auto_impl;

#[auto_impl(Box, &)]
trait Supi {}

#[auto_impl(Box, &)]
trait Foo: Supi {}


fn main() {}
