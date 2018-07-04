use proc_macro::{
    TokenStream,
};
use proc_macro2::{
    TokenStream as TokenStream2,
};
use quote::{TokenStreamExt};
use syn::{GenericParam, TypeParam, LifetimeDef};


use crate::proxy::ProxyType;


fn proxy_ty_param_ident() -> TokenStream2 {
    quote! { __ProxyT }
}

fn proxy_lifetime_ident() -> TokenStream2 {
    quote! { '__proxy_lifetime }
}

pub(crate) fn gen_impls(
    proxy_types: &[ProxyType],
    trait_def: &syn::ItemTrait,
) -> Result<TokenStream, ()> {
    // One impl for each proxy type
    let tokens = proxy_types.into_iter().map(|proxy_type| {
        let header = header(proxy_type, trait_def);

        let out = quote! {
            #header {
                // TODO
            }
        };

        TokenStream::from(out)
    }).collect();

    Ok(tokens)
}

fn header(proxy_type: &ProxyType, trait_def: &syn::ItemTrait) -> TokenStream2 {
    let proxy_ty_param = proxy_ty_param_ident();

    // The tokens after `for` in the impl header
    let self_ty = match *proxy_type {
        ProxyType::Ref      => quote! { &'a #proxy_ty_param },
        ProxyType::RefMut   => quote! { &'a mut #proxy_ty_param },
        ProxyType::Arc      => quote! { ::std::sync::Arc<#proxy_ty_param> },
        ProxyType::Rc       => quote! { ::std::rc::Rc<#proxy_ty_param> },
        ProxyType::Box      => quote! { ::std::boxed::Box<#proxy_ty_param> },
        ProxyType::Fn       => proxy_ty_param.clone(),
        ProxyType::FnMut    => proxy_ty_param.clone(),
        ProxyType::FnOnce   => proxy_ty_param.clone(),
    };

    let trait_params = {
        let mut params = TokenStream2::new();
        for trait_param in &trait_def.generics.params {
            match trait_param {
                GenericParam::Type(TypeParam { ident, ..}) => {
                    params.append_all(quote! { #ident , });
                }
                GenericParam::Lifetime(LifetimeDef { lifetime, ..}) => {
                    params.append_all(quote! { #lifetime , });
                }
                GenericParam::Const(_) => {
                    unimplemented!()
                }
            }
        }

        params
    };

    let where_bounds = TokenStream2::new();


    // Here we assemble the parameter list of the impl (the thing in
    // `impl< ... >`). This is simply the parameter list of the trait with
    // one or two parameter added. For a trait `trait Foo<'x, 'y, A, B>`,
    // it will look like this:
    //
    //    '__ProxyLifetime, 'x, 'y, A, B, __ProxyT
    //
    // The `'__ProxyLifetime` in the beginning is only added when the proxy
    // type is `&` or `&mut`.
    let impl_params = {
        // Determine if our proxy type needs a lifetime parameter
        let mut params = if proxy_type.is_ref() {
            let lifetime_ident = proxy_lifetime_ident();
            quote! { #lifetime_ident, }
        } else {
            quote! {}
        };

        // Append all parameters from the trait
        for trait_param in &trait_def.generics.params {
            match trait_param {
                GenericParam::Type(TypeParam { ident, bounds, ..}) => {
                    params.append_all(quote! { #ident : #bounds , });
                }
                GenericParam::Lifetime(lifetime) => {
                    // TODO: think about attributes?
                    params.append_all(quote! { #lifetime , });
                }
                GenericParam::Const(_) => {
                    unimplemented!()
                }
            }
        }

        // Append proxy type parameter
        params.append_all(proxy_ty_param);

        params
    };

    let trait_name = &trait_def.ident;

    quote! {
        impl<#impl_params> #trait_name<#trait_params> for #self_ty
        where
            #where_bounds
    }
}
