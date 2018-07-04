use proc_macro2::{
    TokenStream as TokenStream2,
    Span,
};
use quote::{TokenStreamExt, ToTokens};
use syn::{
    Ident, Lifetime,
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

        tokens.append_all(quote! {
            #header {
                // TODO
            }
        });
    }

    Ok(tokens.into())
}

fn header(proxy_type: &ProxyType, trait_def: &syn::ItemTrait) -> TokenStream2 {
    let proxy_ty_param = Ident::new(PROXY_TY_PARAM_NAME, Span::call_site());

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
                let lifetime = &Lifetime::new(PROXY_LT_PARAM_NAME, Span::call_site());
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
