#![recursion_limit="128"]
#![cfg_attr(not(test), feature(proc_macro))]

#[cfg(not(test))]
include!("lib.proc_macro.rs");

#[macro_use]
extern crate quote;
extern crate syn;

mod model;
mod impl_as_ref;
mod impl_fn;

use std::str::FromStr;
use quote::Tokens;
use model::*;

const IMPL_FOR_TRAIT_ERR: &'static str = "expected a list containing any of `Arc`, `Rc`, `Box`, `Fn`, `FnMut` or `FnOnce`";

#[derive(Debug, PartialEq)]
enum ImplForTrait {
    Arc,
    Rc,
    Box,
    Fn,
    FnMut,
    FnOnce,
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
            _ => Err(IMPL_FOR_TRAIT_ERR)?
        }
    }
}

fn parse_impl_types(tokens: Tokens) -> Result<Vec<ImplForTrait>, String> {
    let attr = syn::parse_outer_attr(tokens.as_ref())?;

    let idents: Vec<Result<ImplForTrait, String>> = match attr.value {
        syn::MetaItem::Word(ident) => vec![ImplForTrait::from_str(ident.as_ref())],
        syn::MetaItem::List(_, idents) => {
            idents.into_iter().map(|ident| {
                match ident {
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ident)) => ImplForTrait::from_str(ident.as_ref()),
                    _ => Err(IMPL_FOR_TRAIT_ERR)?
                }
            })
            .collect()
        },
        _ => Err(IMPL_FOR_TRAIT_ERR)?
    };

    idents.into_iter().collect()
}

fn auto_impl_expand(impl_for_traits: &[ImplForTrait], tokens: Tokens) -> Result<Tokens, String> {
    let item = syn::parse_item(tokens.as_ref())?;
    let auto_impl = AutoImpl::try_parse(item)?;

    let impls: Vec<_> = impl_for_traits.iter()
        .map(|impl_for_trait| {
            match *impl_for_trait {
                ImplForTrait::Arc => impl_as_ref::build(&auto_impl, Trait::new("Arc", quote!(::std::sync::Arc))),
                ImplForTrait::Rc => impl_as_ref::build(&auto_impl, Trait::new("Rc", quote!(::std::rc::Rc))),
                ImplForTrait::Box => impl_as_ref::build(&auto_impl, Trait::new("Box", quote!(Box))),
                ImplForTrait::Fn => impl_fn::build(&auto_impl, Trait::new("Fn", quote!(Fn))),
                ImplForTrait::FnMut => impl_fn::build(&auto_impl, Trait::new("FnMut", quote!(FnMut))),
                ImplForTrait::FnOnce => impl_fn::build(&auto_impl, Trait::new("FnOnce", quote!(FnOnce)))
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
        let input = quote!(#[auto_impl(Arc, Box, Fn, FnMut, FnOnce)]);

        let impls = parse_impl_types(input).unwrap();

        assert_eq!(vec![
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

        let impls = parse_impl_types(input).unwrap_err();

        assert_eq!("expected a list containing any of `Arc`, `Rc`, `Box`, `Fn`, `FnMut` or `FnOnce`", &impls);
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
            }
        );

        let derive = quote!(
            impl<TAutoImpl> ItWorks for ::std::sync::Arc<TAutoImpl>
                where TAutoImpl: ItWorks
            {
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String> {
                    self.as_ref().method1(arg1, arg2)
                }
                fn method2(&self) {
                    self.as_ref().method2()
                }
            }

            impl<TAutoImpl> ItWorks for Box<TAutoImpl>
                where TAutoImpl: ItWorks
            {
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String> {
                    self.as_ref().method1(arg1, arg2)
                }
                fn method2(&self) {
                    self.as_ref().method2()
                }
            }
        );

        assert_tokens(&[ImplForTrait::Arc, ImplForTrait::Box], input, derive);
    }

    #[test]
    fn impl_as_ref_associated_types() {
        let input = quote!(
            /// Some docs.
            pub trait ItWorks {
                /// Some docs.
                type Type1: AsRef<[u8]>;

                /// Some docs.
                fn method1(&self, arg1: i32, arg2: Self::Type1) -> Result<(), String>;
                /// Some docs.
                fn method2(&self) {
                    println!("default");
                }
            }
        );

        let derive = quote!(
            impl<TAutoImpl> ItWorks for ::std::sync::Arc<TAutoImpl>
                where TAutoImpl: ItWorks
            {
                type Type1 = TAutoImpl::Type1;

                fn method1(&self, arg1: i32, arg2: Self::Type1) -> Result<(), String> {
                    self.as_ref().method1(arg1, arg2)
                }
                fn method2(&self) {
                    self.as_ref().method2()
                }
            }

            impl<TAutoImpl> ItWorks for Box<TAutoImpl>
                where TAutoImpl: ItWorks
            {
                type Type1 = TAutoImpl::Type1;

                fn method1(&self, arg1: i32, arg2: Self::Type1) -> Result<(), String> {
                    self.as_ref().method1(arg1, arg2)
                }
                fn method2(&self) {
                    self.as_ref().method2()
                }
            }
        );

        assert_tokens(&[ImplForTrait::Arc, ImplForTrait::Box], input, derive);
    }

    #[test]
    fn invalid_as_ref_mut_method() {
        let input = quote!(
            pub trait ItWorks {
                fn method1(&mut self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
                fn method2(&self);
            }
        );

        assert_invalid(&[ImplForTrait::Arc], input, "auto impl for `Arc` is only supported for methods with a `&self` reciever");
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
                where TFn: Fn(i32, Option<String>) -> Result<&'static str, String>
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
                where TFn: Fn(i32, &U, &'static str) -> Result<T, String>,
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
                where TFn: FnMut(i32, Option<String>) -> Result<(), String>
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
                where TFn: FnOnce(i32, Option<String>) -> Result<(), String>
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
                where TFn: Fn(i32, Option<String>)
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
            pub trait ItWorks {
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
            pub trait ItWorks {
                fn method<'a>(&'a self) -> Result<Option<&'a str>, String>;
            }
        );

        assert_invalid(&[ImplForTrait::Fn], input, "auto impl for `Fn` is not supported for non-static lifetimes in return types");
    }

    #[test]
    fn invalid_fn_lifetime_in_return_type_tuple() {
        let input = quote!(
            pub trait ItWorks {
                fn method<'a>(&'a self) -> Result<(&'a str, i32), String>;
            }
        );

        assert_invalid(&[ImplForTrait::Fn], input, "auto impl for `Fn` is not supported for non-static lifetimes in return types");
    }

    #[test]
    fn invalid_fn_no_methods() {
        let input = quote!(
            pub trait ItWorks {
                
            }
        );

        assert_invalid(&[ImplForTrait::Fn], input, "auto impl for `Fn` is only supported for traits with 1 method");
    }

    #[test]
    fn invalid_fn_multiple_methods() {
        let input = quote!(
            pub trait ItWorks {
                fn method1(&self, arg1: i32, arg2: Option<String>) -> Result<(), String>;
                fn method2(&self) -> String;
            }
        );

        assert_invalid(&[ImplForTrait::Fn], input, "auto impl for `Fn` is only supported for traits with 1 method");
    }
}
