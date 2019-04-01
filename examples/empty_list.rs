use auto_impl::auto_impl;

#[auto_impl()]
trait Foo {
    fn is_requires(name: String) -> bool;
}