#![recursion_limit="128"]
#![cfg_attr(not(test), feature(proc_macro))]

#[cfg(not(test))]
include!("lib.proc_macro.rs");

#[macro_use]
extern crate quote;
extern crate syn;

mod parse;
mod model;
mod impl_as_ref;
mod impl_fn;

use std::str::FromStr;
use quote::Tokens;
use impl_as_ref::{RefTrait, WrapperTrait};
use impl_fn::FnTrait;
use model::*;

const IMPL_FOR_TRAIT_ERR: &'static str = "expected a list containing any of `&`, `&mut`, `Arc`, `Rc`, `Box`, `Fn`, `FnMut` or `FnOnce`";

#[derive(Debug, PartialEq)]
enum ImplForTrait {
    Arc,
    Rc,
    Box,
    Fn,
    FnMut,
    FnOnce,
    Ref,
    RefMut,
}

impl FromStr for ImplForTrait {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Arc" => Ok(ImplForTrait::Arc),
            "Rc" => Ok(ImplForTrait::Rc),
            "Box" => Ok(ImplForTrait::Box),
            "Fn" => Ok(ImplForTrait::Fn),
            "FnMut" => Ok(ImplForTrait::FnMut),
            "FnOnce" => Ok(ImplForTrait::FnOnce),
            "&" => Ok(ImplForTrait::Ref),
            "&mut" => Ok(ImplForTrait::RefMut),
            _ => Err(IMPL_FOR_TRAIT_ERR)?,
        }
    }
}

fn parse_impl_types(tokens: Tokens) -> Result<Vec<ImplForTrait>, String> {
    parse::attr(tokens.as_str())?
        .into_iter()
        .map(|ident| ImplForTrait::from_str(&ident))
        .collect()
}

fn auto_impl_expand(impl_for_traits: &[ImplForTrait], tokens: Tokens) -> Result<Tokens, String> {
    let item = syn::parse_item(tokens.as_ref())?;
    let auto_impl = AutoImpl::try_parse(item)?;

    let impls: Vec<_> = impl_for_traits.iter()
        .map(|impl_for_trait| {
            match *impl_for_trait {
                ImplForTrait::Arc => impl_as_ref::build_wrapper(&auto_impl, WrapperTrait::impl_arc()),
                ImplForTrait::Rc => impl_as_ref::build_wrapper(&auto_impl, WrapperTrait::impl_rc()),
                ImplForTrait::Box => impl_as_ref::build_wrapper(&auto_impl, WrapperTrait::impl_box()),
                ImplForTrait::Ref => impl_as_ref::build_ref(&auto_impl, RefTrait::impl_ref()),
                ImplForTrait::RefMut => impl_as_ref::build_ref(&auto_impl, RefTrait::impl_ref_mut()),
                ImplForTrait::Fn => impl_fn::build(&auto_impl, FnTrait::impl_fn()),
                ImplForTrait::FnMut => impl_fn::build(&auto_impl, FnTrait::impl_fn_mut()),
                ImplForTrait::FnOnce => impl_fn::build(&auto_impl, FnTrait::impl_fn_once())
            }
        })
        .collect();

    if let Some(err) = impls.iter().find(|res| res.is_err()) {
        let err = err.clone().unwrap_err();
        return Err(err)
    }

    let impls: Vec<_> = impls.into_iter()
        .filter_map(Result::ok)
        .collect();

    Ok(quote!(
        #tokens
        
        #(#impls)*
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_tokens(impl_for_traits: &[ImplForTrait], input: Tokens, derive: Tokens) {
        let mut expected = Tokens::new();
        expected.append_all(&[&input, &derive]);

        let actual = auto_impl_expand(impl_for_traits, input).unwrap();

        assert_eq!(expected.to_string(), actual.to_string());
    }

    fn assert_invalid(impl_for_traits: &[ImplForTrait], input: Tokens, err: &'static str) {
        let actual_err = auto_impl_expand(impl_for_traits, input).unwrap_err();

        assert_eq!(err, &actual_err);
    }

    #[test]
    fn impl_types() {
        let input = quote!(#[auto_impl(&, &mut, Rc, Arc, Box, Fn, FnMut, FnOnce)]);

        let impls = parse_impl_types(input).unwrap();

        assert_eq!(vec![
            ImplForTrait::Ref,
            ImplForTrait::RefMut,
            ImplForTrait::Rc,
            ImplForTrait::Arc,
            ImplForTrait::Box,
            ImplForTrait::Fn,
            ImplForTrait::FnMut,
            ImplForTrait::FnOnce
        ], impls);
    }

    #[test]
    fn invalid_impl_types() {
        let input = quote!(#[auto_impl(NotSupported)]);

        let impls = parse_impl_types(input);

        assert!(impls.is_err());
    }

    #[test]
    fn parse_attr_raw_single() {
        let input = "#[auto_impl(&)]";
        let parsed = parse::attr(input).unwrap();

        assert_eq!(parsed, &["&"]);
    }

    #[test]
    fn parse_attr_raw() {
        let input = "#[auto_impl(&, &mut, Arc)]";
        let parsed = parse::attr(input).unwrap();

        assert_eq!(parsed, &["&", "&mut", "Arc"]);
    }

    #[test]
    fn impl_as_ref() {
        let input = quote!(
            /// Some docs.
            pub trait ItWorks {
                /// Some docs.
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
                /// Some docs.
                fn method2(&self) {
                    println!("default");
                }
                /// Some docs.
                fn method3() -> &'static str;
            }
        );

        let derive = quote!(
            impl<TAutoImpl> ItWorks for ::std::sync::Arc<TAutoImpl>
                where TAutoImpl: ItWorks
            {
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String> {
                    (**self).method1(arg1, arg2)
                }
                fn method2(&self) {
                    (**self).method2()
                }
                fn method3() -> &'static str {
                    TAutoImpl::method3()
                }
            }

            impl<TAutoImpl> ItWorks for ::std::rc::Rc<TAutoImpl>
                where TAutoImpl: ItWorks
            {
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String> {
                    (**self).method1(arg1, arg2)
                }
                fn method2(&self) {
                    (**self).method2()
                }
                fn method3() -> &'static str {
                    TAutoImpl::method3()
                }
            }
        );

        assert_tokens(&[ImplForTrait::Arc, ImplForTrait::Rc], input, derive);
    }

    #[test]
    fn impl_box() {
        let input = quote!(
            /// Some docs.
            pub trait ItWorks {
                /// Some docs.
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
                /// Some docs.
                fn method2(&mut self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
                /// Some docs.
                fn method3(self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
                /// Some docs.
                fn method4() -> &'static str;
            }
        );

        let derive = quote!(
            impl<TAutoImpl> ItWorks for ::std::boxed::Box<TAutoImpl>
                where TAutoImpl: ItWorks
            {
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String> {
                    (**self).method1(arg1, arg2)
                }
                fn method2(&mut self, arg1: i32, arg2: Option<String>) -> Result<(), String> {
                    (**self).method2(arg1, arg2)
                }
                fn method3(self, arg1: i32, arg2: Option<String>) -> Result<(), String> {
                    (*self).method3(arg1, arg2)
                }
                fn method4() -> &'static str {
                    TAutoImpl::method4()
                }
            }
        );

        assert_tokens(&[ImplForTrait::Box], input, derive);
    }

    #[test]
    fn impl_as_ref_associated_types() {
        let input = quote!(
            /// Some docs.
            pub trait ItWorks {
                /// Some docs.
                type Type1: AsRef<[u8]>;
            }
        );

        let derive = quote!(
            impl<TAutoImpl> ItWorks for ::std::sync::Arc<TAutoImpl>
                where TAutoImpl: ItWorks
            {
                type Type1 = TAutoImpl::Type1;
            }
        );

        assert_tokens(&[ImplForTrait::Arc], input, derive);
    }

    #[test]
    fn invalid_as_ref_mut_method() {
        let input = quote!(
            pub trait ItWorks {
                fn method1(&mut self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
            }
        );

        assert_invalid(&[ImplForTrait::Arc], input, "auto impl for `Arc` is only supported for methods with a `&self` or static reciever");
    }

    #[test]
    fn invalid_as_ref_by_value_method() {
        let input = quote!(
            pub trait ItWorks {
                fn method1(self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
            }
        );

        assert_invalid(&[ImplForTrait::Arc], input, "auto impl for `Arc` is only supported for methods with a `&self` or static reciever");
    }

    #[test]
    fn impl_ref() {
        let input = quote!(
            /// Some docs.
            pub trait ItWorks {
                /// Some docs.
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
            }
        );

        let derive = quote!(
            impl<'auto, TAutoImpl> ItWorks for &'auto TAutoImpl
                where TAutoImpl: ItWorks
            {
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String> {
                    (**self).method1(arg1, arg2)
                }
            }

            impl<'auto, TAutoImpl> ItWorks for &'auto mut TAutoImpl
                where TAutoImpl: ItWorks
            {
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String> {
                    (**self).method1(arg1, arg2)
                }
            }
        );

        assert_tokens(&[ImplForTrait::Ref, ImplForTrait::RefMut], input, derive);
    }

    #[test]
    fn invalid_ref_mut_method() {
        let input = quote!(
            pub trait ItFails {
                fn method1(&mut self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
            }
        );

        assert_invalid(&[ImplForTrait::Ref], input, "auto impl for `&T` is only supported for methods with a `&self` or static reciever");
    }

    #[test]
    fn invalid_ref_by_value_method() {
        let input = quote!(
            pub trait ItFails {
                fn method1(self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
            }
        );

        assert_invalid(&[ImplForTrait::Ref], input, "auto impl for `&T` is only supported for methods with a `&self` or static reciever");
    }

    #[test]
    fn invalid_ref_mut_by_value_method() {
        let input = quote!(
            pub trait ItFails {
                fn method1(self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
            }
        );

        assert_invalid(&[ImplForTrait::RefMut], input, "auto impl for `&mut T` is only supported for methods with a `&self`, `&mut self` or static reciever");
    }

    #[test]
    fn impl_fn() {
        let input = quote!(
            /// Some docs.
            pub trait ItWorks {
                /// Some docs.
                fn method(&self, arg1: i32, arg2: Option<String>) -> Result<&'static str, String>;
            }
        );

        let derive = quote!(
            impl<TFn> ItWorks for TFn
                where TFn: ::std::ops::Fn(i32, Option<String>) -> Result<&'static str, String>
            {
                fn method(&self, arg1: i32, arg2: Option<String>) -> Result<&'static str, String> {
                    self(arg1, arg2)
                }
            }
        );

        assert_tokens(&[ImplForTrait::Fn], input, derive);
    }

    #[test]
    fn impl_fn_generics() {
        let input = quote!(
            /// Some docs.
            pub trait ItWorks<'a, T, U> where U: AsRef<[u8]> {
                /// Some docs.
                fn method<'b>(&'a self, arg1: i32, arg2: &'b U, arg3: &'static str) -> Result<T, String>;
            }
        );

        let derive = quote!(
            impl<'a, T, U, TFn> ItWorks<'a, T, U> for TFn
                where TFn: ::std::ops::Fn(i32, &U, &'static str) -> Result<T, String>,
                      U: AsRef<[u8]>
            {
                fn method<'b>(&'a self, arg1: i32, arg2: &'b U, arg3: &'static str) -> Result<T, String> {
                    self(arg1, arg2, arg3)
                }
            }
        );

        assert_tokens(&[ImplForTrait::Fn], input, derive);
    }

    #[test]
    fn impl_fn_mut() {
        let input = quote!(
            pub trait ItWorks {
                fn method(&mut self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
            }
        );

        let derive = quote!(
            impl<TFn> ItWorks for TFn
                where TFn: ::std::ops::FnMut(i32, Option<String>) -> Result<(), String>
            {
                fn method(&mut self, arg1: i32, arg2: Option<String>) -> Result<(), String> {
                    self(arg1, arg2)
                }
            }
        );

        assert_tokens(&[ImplForTrait::FnMut], input, derive);
    }

    #[test]
    fn impl_fn_once() {
        let input = quote!(
            pub trait ItWorks {
                fn method(self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
            }
        );

        let derive = quote!(
            impl<TFn> ItWorks for TFn
                where TFn: ::std::ops::FnOnce(i32, Option<String>) -> Result<(), String>
            {
                fn method(self, arg1: i32, arg2: Option<String>) -> Result<(), String> {
                    self(arg1, arg2)
                }
            }
        );

        assert_tokens(&[ImplForTrait::FnOnce], input, derive);
    }

    #[test]
    fn impl_fn_no_return() {
        let input = quote!(
            pub trait ItWorks {
                fn method(&self, arg1: i32, arg2: Option<String>);
            }
        );

        let derive = quote!(
            impl<TFn> ItWorks for TFn
                where TFn: ::std::ops::Fn(i32, Option<String>)
            {
                fn method(&self, arg1: i32, arg2: Option<String>) {
                    self(arg1, arg2)
                }
            }
        );

        assert_tokens(&[ImplForTrait::Fn], input, derive);
    }

    #[test]
    fn invalid_fn_associated_types() {
        let input = quote!(
            pub trait ItFails {
                type TypeA;
                type TypeB;

                fn method(&self);
            }
        );

        assert_invalid(&[ImplForTrait::Fn], input, "auto impl for `Fn` is not supported for associated types");
    }

    #[test]
    fn invalid_fn_lifetime_in_return_type_path() {
        let input = quote!(
            pub trait ItFails {
                fn method<'a>(&'a self) -> Result<Option<&'a str>, String>;
            }
        );

        assert_invalid(&[ImplForTrait::Fn], input, "auto impl for `Fn` is not supported for non-static lifetimes in return types");
    }

    #[test]
    fn invalid_fn_lifetime_in_return_type_tuple() {
        let input = quote!(
            pub trait ItFails {
                fn method<'a>(&'a self) -> Result<(&'a str, i32), String>;
            }
        );

        assert_invalid(&[ImplForTrait::Fn], input, "auto impl for `Fn` is not supported for non-static lifetimes in return types");
    }

    #[test]
    fn invalid_fn_no_methods() {
        let input = quote!(
            pub trait ItFails {
                
            }
        );

        assert_invalid(&[ImplForTrait::Fn], input, "auto impl for `Fn` is only supported for traits with 1 method");
    }

    #[test]
    fn invalid_fn_multiple_methods() {
        let input = quote!(
            pub trait ItFails {
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
                fn method2(&self) -> String;
            }
        );

        assert_invalid(&[ImplForTrait::Fn], input, "auto impl for `Fn` is only supported for traits with 1 method");
    }
}
