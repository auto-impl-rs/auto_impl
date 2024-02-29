use auto_impl::auto_impl;

#[auto_impl(Box, &)]
trait Supi {
    type Assoc;
}

#[auto_impl(Box, &)]
trait Foo: Supi {
    fn foo(&self, x: Self::Assoc) -> String;
}


fn main() {}
