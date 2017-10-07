# `auto_impl` [![Build Status](https://travis-ci.org/KodrAus/auto_impl.svg?branch=master)](https://travis-ci.org/KodrAus/auto_impl) [![Crates.io](https://img.shields.io/crates/v/auto_impl.svg)](https://crates.io/crates/auto_impl)

A simple compiler plugin for automatically implementing a trait for some common smart pointers and closures.

# Usage

This library requires the `nightly` channel.

Add `auto_impl` to your `Cargo.toml`:

```
auto_impl = "*"
```

Reference in your crate root:

```rust
#![feature(proc_macro)]

extern crate auto_impl;

use auto_impl::auto_impl;
```

The following types are supported:

- [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)
- [`Rc`](https://doc.rust-lang.org/std/rc/struct.Rc.html)
- [`Box`](https://doc.rust-lang.org/std/boxed/struct.Box.html)
- [`Fn`](https://doc.rust-lang.org/std/ops/trait.Fn.html)
- [`FnMut`](https://doc.rust-lang.org/std/ops/trait.FnMut.html)
- [`FnOnce`](https://doc.rust-lang.org/std/ops/trait.FnOnce.html)
- `&`
- `&mut`

## Implement a trait for a smart pointer

Add the `#[auto_impl]` attribute to traits to automatically implement them for wrapper types:

```rust
#[auto_impl(Arc, Box, Rc)]
trait MyTrait<'a, T> 
    where T: AsRef<str>
{
    type Type1;
    type Type2;

    fn execute1<'b>(&'a self, arg1: &'b T) -> Result<Self::Type1, String>;
    fn execute2(&self) -> Self::Type2;
}
```

Will expand to:

```rust
impl<'a, T, TAutoImpl> MyTrait<'a, T> for ::std::sync::Arc<TAutoImpl>
    where TAutoImpl: MyTrait<'a, T>,
          T: AsRef<str>
{
    type Type1 = TAutoImpl::Type1;
    type Type2 = TAutoImpl::Type2;

    fn execute1<'b>(&'a self, arg1: &'b T) -> Result<Self::Type1, String> {
        self.as_ref().execute1(arg1)
    }

    fn execute2(&self) -> Self::Type2 {
        self.as_ref().execute2()
    }
}

impl<'a, T, TAutoImpl> MyTrait<'a, T> for Box<TAutoImpl>
    where TAutoImpl: MyTrait<'a, T>,
          T: AsRef<str>
{
    type Type1 = TAutoImpl::Type1;
    type Type2 = TAutoImpl::Type2;

    fn execute1<'b>(&'a self, arg1: &'b T) -> Result<Self::Type1, String> {
        self.as_ref().execute1(arg1)
    }

    fn execute2(&self) -> Self::Type2 {
        self.as_ref().execute2()
    }
}

impl<'a, T, TAutoImpl> MyTrait<'a, T> for ::std::rc::Rc<TAutoImpl>
    where TAutoImpl: MyTrait<'a, T>,
          T: AsRef<str>
{
    type Type1 = TAutoImpl::Type1;
    type Type2 = TAutoImpl::Type2;

    fn execute1<'b>(&'a self, arg1: &'b T) -> Result<Self::Type1, String> {
        self.as_ref().execute1(arg1)
    }

    fn execute2(&self) -> Self::Type2 {
        self.as_ref().execute2()
    }
}
```

There are a few restrictions on `#[auto_impl]` for smart pointers. The trait must:

- Only have methods that take `&self`

## Implement a trait for a closure

```rust
#[auto_impl(Fn)]
trait MyTrait<'a, T> {
    fn execute<'b>(&'a self, arg1: &'b T, arg2: &'static str) -> Result<(), String>;
}
```

Will expand to:

```rust
impl<'a, T, TAutoImpl> MyTrait<'a, T> for TAutoImpl
    where TAutoImpl: Fn(&T, &'static str) -> Result<(), String>
{
    fn execute<'b>(&'a self, arg1: &'b T, arg1: &'static str) -> Result<(), String> {
        self(arg1, arg2)
    }
}
```

There are a few restrictions on `#[auto_impl]` for closures. The trait must:

- Have a single method
- Have no associated types
- Have no non-static lifetimes in the return type

## Implement a trait for a borrowed reference

```rust
#[auto_impl(&, &mut)]
trait MyTrait<'a, T> {
    fn execute<'b>(&'a self, arg1: &'b T, arg2: &'static str) -> Result<(), String>;
}
```

Will expand to:

```rust
impl<'auto, 'a, T, TAutoImpl> MyTrait<'a, T> for &'auto TAutoImpl {
    fn execute<'b>(&'a self, arg1: &'b T, arg1: &'static str) -> Result<(), String> {
        (**self).execute(arg1, arg2)
    }
}

impl<'auto, 'a, T, TAutoImpl> MyTrait<'a, T> for &'auto mut TAutoImpl {
    fn execute<'b>(&'a self, arg1: &'b T, arg1: &'static str) -> Result<(), String> {
        (**self).execute(arg1, arg2)
    }
}
```

There are a few restrictions on `#[auto_impl]` for immutably borrowed references. The trait must:

- Only have methods that take `&self`

There are no restrictions on `#[auto_impl]` for mutably borrowed references.
