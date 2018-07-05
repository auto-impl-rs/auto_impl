use proc_macro::Span;
use proc_macro2::{
    TokenStream as TokenStream2,
    Span as Span2,
};
use quote::{TokenStreamExt, ToTokens};
use syn::{
    Ident, Lifetime, ItemTrait, TraitItem, TraitItemMethod, FnArg, Pat, PatIdent,
    TraitItemType,
};


use crate::{
    proxy::ProxyType,
    spanned::Spanned,
};

/// The type parameter used in the proxy type. Usually, one would just use `T`,
/// but this could conflict with type parameters on the trait.
///
/// Why do we have to care about this? Why about hygiene? In the first version
/// of stable proc_macros, only call site spans are included. That means that
/// we cannot generate spans that do not conflict with any other ident the user
/// wrote. Once proper hygiene is available to proc_macros, this should be
/// changed.
const PROXY_TY_PARAM_NAME: &str = "__AutoImplProxyT";

/// The lifetime parameter used in the proxy type if the proxy type is `&` or
/// `&mut`. For more information see `PROXY_TY_PARAM_NAME`.
const PROXY_LT_PARAM_NAME: &str = "'__auto_impl_proxy_lifetime";


/// Generates one complete impl of the given trait for each of the given proxy
/// types. All impls are returned as token stream.
pub(crate) fn gen_impls(
    proxy_types: &[ProxyType],
    trait_def: &syn::ItemTrait,
) -> Result<::proc_macro::TokenStream, ()> {
    let mut tokens = TokenStream2::new();

    // One impl for each proxy type
    for proxy_type in proxy_types {
        let header = header(proxy_type, trait_def)?;
        let items = gen_items(proxy_type, trait_def)?;

        tokens.append_all(quote! {
            #header { #( #items )* }
        });
    }

    Ok(tokens.into())
}

/// Generates the header of the impl of the given trait for the given proxy
/// type.
fn header(proxy_type: &ProxyType, trait_def: &ItemTrait) -> Result<TokenStream2, ()> {
    let proxy_ty_param = Ident::new(PROXY_TY_PARAM_NAME, Span2::call_site());

    // Generate generics for impl positions from trait generics.
    let (impl_generics, trait_generics, where_clause) = trait_def.generics.split_for_impl();

    // The name of the trait with all generic parameters applied.
    let trait_ident = &trait_def.ident;
    let trait_path = quote! { #trait_ident #trait_generics };


    // Here we assemble the parameter list of the impl (the thing in
    // `impl< ... >`). This is simply the parameter list of the trait with
    // one or two parameters added. For a trait `trait Foo<'x, 'y, A, B>`,
    // it will look like this:
    //
    //    '__auto_impl_proxy_lifetime, 'x, 'y, A, B, __AutoImplProxyT
    //
    // The `'__auto_impl_proxy_lifetime` in the beginning is only added when
    // the proxy type is `&` or `&mut`.
    let impl_generics = {
        // Determine if our proxy type needs a lifetime parameter
        let (mut params, ty_bounds) = match proxy_type {
            ProxyType::Ref | ProxyType::RefMut => {
                let lifetime = &Lifetime::new(PROXY_LT_PARAM_NAME, Span2::call_site());
                (quote! { #lifetime, }, quote! { : #lifetime + #trait_path })
            }
            ProxyType::Box | ProxyType::Rc | ProxyType::Arc => {
                (quote! {}, quote! { : #trait_path })
            }
            _ => unimplemented!(),
        };

        // Append all parameters from the trait. Sadly, `impl_generics`
        // includes the angle brackets `< >` so we have to remove them like
        // this.
        let mut tts = impl_generics.into_token_stream()
            .into_iter()
            .skip(1)    // the opening `<`
            .collect::<Vec<_>>();
        tts.pop();  // the closing `>`
        params.append_all(tts);

        // Append proxy type parameter (if there aren't any parameters so far,
        // we need to add a comma first).
        let comma = if params.is_empty() {
            quote! {}
        } else {
            quote! { , }
        };
        params.append_all(quote! { #comma #proxy_ty_param #ty_bounds });

        params
    };


    // The tokens after `for` in the impl header (the type the trait is
    // implemented for).
    let self_ty = match *proxy_type {
        ProxyType::Ref      => quote! { &'a #proxy_ty_param },
        ProxyType::RefMut   => quote! { &'a mut #proxy_ty_param },
        ProxyType::Arc      => quote! { ::std::sync::Arc<#proxy_ty_param> },
        ProxyType::Rc       => quote! { ::std::rc::Rc<#proxy_ty_param> },
        ProxyType::Box      => quote! { ::std::boxed::Box<#proxy_ty_param> },
        ProxyType::Fn       => quote! { #proxy_ty_param },
        ProxyType::FnMut    => quote! { #proxy_ty_param },
        ProxyType::FnOnce   => quote! { #proxy_ty_param },
    };


    // Combine everything
    Ok(quote! {
        impl<#impl_generics> #trait_path for #self_ty #where_clause
    })
}

/// Generates the implementation of all items of the given trait. These
/// implementations together are the body of the `impl` block.
fn gen_items(
    proxy_type: &ProxyType,
    trait_def: &ItemTrait,
) -> Result<Vec<TokenStream2>, ()> {
    trait_def.items.iter().map(|item| {
        match item {
            TraitItem::Const(_) => unimplemented!(),
            TraitItem::Method(method) => gen_method_item(proxy_type, method, trait_def),
            TraitItem::Type(ty) => gen_type_item(proxy_type, ty, trait_def),
            TraitItem::Macro(mac) => {
                // We cannot resolve the macro invocation and thus cannot know
                // if it adds additional items to the trait. Thus, we have to
                // give up.
                mac.span()
                    .error("\
                        traits with macro invocations in their bodies are not \
                        supported by auto_impl\
                    ")
                    .span_note(Span::call_site(), "auto-impl requested here")
                    .emit();

                Err(())
            },
            TraitItem::Verbatim(v) => {
                // I don't quite know when this happens, but it's better to
                // notify the user with a nice error instead of panicking.
                v.span()
                    .error("unexpected 'verbatim'-item (auto-impl doesn't know how to handle it)")
                    .span_note(Span::call_site(), "auto-impl requested here")
                    .emit();

                Err(())
            }
        }
    }).collect()
}

/// Generates the implementation of a associated type item described by `item`.
/// The implementation is returned as token stream.
///
/// If the proxy type is an Fn*-trait, an error is emitted and `Err(())` is
/// returned.
fn gen_type_item(
    proxy_type: &ProxyType,
    item: &TraitItemType,
    trait_def: &ItemTrait,
) -> Result<TokenStream2, ()> {
    // A trait with associated types cannot be implemented for Fn* types.
    if proxy_type.is_fn() {
        let msg = format!(
            "the trait `{}` cannot be auto-implemented for Fn-traits, because it has associated \
                types (only traits with a single method can be implemented for Fn-traits)",
            trait_def.ident,
        );

        item.span()
            .error(msg)
            .span_note(Span::call_site(), "auto-impl for Fn-trait requested here")
            .emit();

        return Err(());
    }

    // We simply use the associated type from our type parameter.
    let assoc_name = &item.ident;
    let proxy_ty_param = Ident::new(PROXY_TY_PARAM_NAME, Span2::call_site());

    Ok(quote ! {
        type #assoc_name = #proxy_ty_param::#assoc_name;
    })
}

/// Generates the implementation of a method item described by `item`. The
/// implementation is returned as token stream.
///
/// This function also performs sanity checks, e.g. whether the proxy type can
/// be used to implement the method. If any error occurs, the error is
/// immediately emitted and `Err(())` is returned.
fn gen_method_item(
    proxy_type: &ProxyType,
    item: &TraitItemMethod,
    trait_def: &ItemTrait,
) -> Result<TokenStream2, ()> {
    // Determine the kind of the method, determined by the self type.
    let sig = &item.sig;
    let self_arg = match sig.decl.inputs.iter().next() {
        Some(FnArg::SelfValue(_)) => SelfType::Value,
        Some(FnArg::SelfRef(arg)) if arg.mutability.is_none() => SelfType::Ref,
        Some(FnArg::SelfRef(arg)) if arg.mutability.is_some() => SelfType::Mut,
        _ => SelfType::None,
    };

    // Check self type and proxy type combination
    check_receiver_compatible(proxy_type, self_arg, &trait_def.ident, sig.span())?;

    // Generate the list of argument used to call the method.
    let args = get_arg_list(sig.decl.inputs.iter())?;

    // Generate the body of the function. This mainly depends on the self type,
    // but also on the proxy type.
    let name = &sig.ident;
    let body = match self_arg {
        // No receiver
        SelfType::None => {
            // The proxy type is a reference, smartpointer or Box, but not Fn*.
            let proxy_ty_param = Ident::new(PROXY_TY_PARAM_NAME, Span2::call_site());
            quote! { #proxy_ty_param::#name(#args) }
        }

        // Receiver `self` (by value)
        SelfType::Value => {
            // The proxy type is either Box or Fn*.
            if *proxy_type == ProxyType::Box {
                quote! { (*self).#name(#args) }
            } else {
                unimplemented!()
            }
        }

        // `&self` or `&mut self` receiver
        SelfType::Ref | SelfType::Mut => {
            // The proxy type could be anything in the `Ref` case, and `&mut`,
            // Box or Fn* in the `Mut` case.
            if proxy_type.is_fn() {
                unimplemented!()
            } else {
                quote! { (**self).#name(#args) }
            }
        }
    };

    // Combine body with signature
    // TODO: maybe add `#[inline]`?
    Ok(quote! { #sig { #body }})
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelfType {
    None,
    Ref,
    Mut,
    Value,
}

impl SelfType {
    fn as_str(&self) -> Option<&'static str> {
        match *self {
            SelfType::None => None,
            SelfType::Ref => Some("&self"),
            SelfType::Mut => Some("&mut self"),
            SelfType::Value => Some("self"),
        }
    }
}

/// Checks if this method can be implemented for the given proxy type. If not,
/// we will emit an error pointing to the method signature.
fn check_receiver_compatible(
    proxy_type: &ProxyType,
    self_arg: SelfType,
    trait_name: &Ident,
    sig_span: Span,
) -> Result<(), ()> {
    match (proxy_type, self_arg) {
        (ProxyType::Ref, SelfType::Mut) |
        (ProxyType::Ref, SelfType::Value) => {
            let msg = format!(
                "the trait `{}` cannot be auto-implemented for immutable references, because \
                    this method has a `{}` receiver (only `&self` and no receiver are allowed)",
                trait_name,
                self_arg.as_str().unwrap(),
            );

            sig_span
                .error(msg)
                .span_note(Span::call_site(), "auto-impl for immutable references requested here")
                .emit();

            Err(())
        }

        (ProxyType::RefMut, SelfType::Value) => {
            let msg = format!(
                "the trait `{}` cannot be auto-implemented for mutable references, because \
                    this method has a `self` receiver (only `&self`, `&mut self` and no receiver \
                    are allowed)",
                trait_name,
            );

            sig_span
                .error(msg)
                .span_note(Span::call_site(), "auto-impl for mutable references requested here")
                .emit();

            Err(())
        }

        (ProxyType::Rc, SelfType::Mut) |
        (ProxyType::Rc, SelfType::Value) |
        (ProxyType::Arc, SelfType::Mut) |
        (ProxyType::Arc, SelfType::Value) => {
            let ptr_name = if *proxy_type == ProxyType::Rc {
                "Rc"
            } else {
                "Arc"
            };

            let msg = format!(
                "the trait `{}` cannot be auto-implemented for {}-smartpointer, because \
                    this method has a `{}` receiver (only `&self` and no receiver are allowed)",
                trait_name,
                ptr_name,
                self_arg.as_str().unwrap(),
            );

            sig_span
                .error(msg)
                .span_note(Span::call_site(), "auto-impl for mutable references requested here")
                .emit();

            Err(())
        }

        (ProxyType::Fn, _) |
        (ProxyType::FnMut, _) |
        (ProxyType::FnOnce, _) => {
            unimplemented!()
        }

        _ => Ok(()), // All other combinations are fine
    }
}

/// Generates a list of comma-separated arguments used to call the function.
/// Currently, only simple names are valid and more complex pattern will lead
/// to an error being emitted. `self` parameters are ignored.
fn get_arg_list(inputs: impl Iterator<Item = &'a FnArg>) -> Result<TokenStream2, ()> {
    let mut args = TokenStream2::new();

    for arg in inputs {
        match arg {
            FnArg::Captured(arg) => {
                // Make sure the argument pattern is a simple name. In
                // principle, we could probably support patterns, but it's
                // not that important now.
                if let Pat::Ident(PatIdent {
                    by_ref: None,
                    mutability: None,
                    ident,
                    subpat: None,
                }) = &arg.pat {
                    // Add name plus trailing comma to tokens
                    args.append_all(quote! { #ident , });
                } else {
                    arg.pat.span()
                        .error("\
                            argument patterns are not supported by #[auto-impl]. Please use \
                            a simple name (not `_`).\
                        ")
                        .emit();

                    return Err(());
                }
            }

            // Honestly, I'm not sure what this is.
            FnArg::Ignored(ty) => {
                ty.span()
                    .error("cannot auto-impl trait, because this argument is ignored")
                    .emit();

                return Err(());
            }

            FnArg::Inferred(_) => {
                // This can't happen in today's Rust and it's unlikely to
                // change in the near future.
                panic!("argument with inferred type in trait method");
            }

            // There is only one such argument. We handle it elsewhere and
            // can ignore it here.
            FnArg::SelfRef(_) | FnArg::SelfValue(_) => {}
        }
    }

    Ok(args)
}
