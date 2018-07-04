use proc_macro2::{
    TokenStream as TokenStream2,
    Span as Span2,
};
use quote::{TokenStreamExt, ToTokens};
use syn::{
    Ident, Lifetime, ItemTrait, TraitItem, TraitItemMethod, FnArg,
};


use crate::proxy::ProxyType;

const PROXY_TY_PARAM_NAME: &str = "__AutoImplProxyT";
const PROXY_LT_PARAM_NAME: &str = "'__auto_impl_proxy_lifetime";


pub(crate) fn gen_impls(
    proxy_types: &[ProxyType],
    trait_def: &syn::ItemTrait,
) -> Result<::proc_macro::TokenStream, ()> {
    let mut tokens = TokenStream2::new();

    // One impl for each proxy type
    for proxy_type in proxy_types {
        let header = header(proxy_type, trait_def);
        let items = items(proxy_type, trait_def)?;

        tokens.append_all(quote! {
            #header { #( #items )* }
        });
    }

    Ok(tokens.into())
}

fn header(proxy_type: &ProxyType, trait_def: &ItemTrait) -> TokenStream2 {
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
    quote! {
        impl<#impl_generics> #trait_path for #self_ty #where_clause
    }
}

fn items(
    proxy_type: &ProxyType,
    trait_def: &ItemTrait,
) -> Result<Vec<TokenStream2>, ()> {
    trait_def.items.iter().map(|item| {
        match item {
            TraitItem::Const(_) => unimplemented!(),
            TraitItem::Method(method) => method_item(proxy_type, method),
            TraitItem::Type(_) => unimplemented!(),
            TraitItem::Macro(_) => unimplemented!(),
            TraitItem::Verbatim(_) => unimplemented!(),
        }
    }).collect()
}

fn method_item(proxy_type: &ProxyType, item: &TraitItemMethod) -> Result<TokenStream2, ()> {
    enum SelfType {
        None,
        Ref,
        Mut,
        Value,
    }

    let sig = &item.sig;
    let name = &sig.ident;
    let args = TokenStream2::new(); // TODO

    let self_arg = match sig.decl.inputs.iter().next() {
        Some(FnArg::SelfValue(_)) => SelfType::Value,
        Some(FnArg::SelfRef(arg)) if arg.mutability.is_none() => SelfType::Ref,
        Some(FnArg::SelfRef(arg)) if arg.mutability.is_some() => SelfType::Mut,
        _ => SelfType::None,
    };

    // Check if this method can be implemented for the given proxy type
    match (proxy_type, self_arg) {
        (ProxyType::Ref, SelfType::Mut) |
        (ProxyType::Ref, SelfType::Value) => {

        }
        (ProxyType::RefMut, SelfType::Value) => {

        }
        (ProxyType::Rc, SelfType::Mut) |
        (ProxyType::Rc, SelfType::Value) |
        (ProxyType::Arc, SelfType::Mut) |
        (ProxyType::Arc, SelfType::Value) => {

        }
        (ProxyType::Fn, _) |
        (ProxyType::FnMut, _) |
        (ProxyType::FnOnce, _) => {
            unimplemented!()
        }
        _ => {} // All other combinations are fine
    }

    let body = match proxy_type {
        ProxyType::Ref | ProxyType::RefMut | ProxyType::Arc | ProxyType::Rc | ProxyType::Box => {
            quote! {
                (**self).#name(#args)
            }
        }
        ProxyType::Fn | ProxyType::FnMut | ProxyType::FnOnce => {
            quote! {
                self(#args)
            }
        }
    };


    Ok(quote! { #sig { #body }})
}
