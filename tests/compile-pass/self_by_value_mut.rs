use auto_impl::auto_impl;


struct Data {
    id: usize,
}

#[auto_impl(Fn)]
trait Foo {
    fn foo(&self, ref mut data: Data) {
        data.id += 1;
    }
}


fn main() {}
