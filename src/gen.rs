use proc_macro::{
    TokenStream,
};
use proc_macro2::TokenStream as TokenStream2;


use crate::proxy::ProxyType;


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
    let trait_name = &trait_def.ident;

    // The tokens after `for` in the impl header
    let self_ty = match *proxy_type {
        ProxyType::Ref => quote! { &'a T },
        ProxyType::RefMut => quote! { &'a mut T },
        ProxyType::Arc => quote! { ::std::sync::Arc<T> },
        ProxyType::Rc => quote! { ::std::rc::Rc<T> },
        ProxyType::Box => quote! { ::std::boxed::Box<T> },
        ProxyType::Fn => unimplemented!(),
        ProxyType::FnMut => unimplemented!(),
        ProxyType::FnOnce => unimplemented!(),
    };

    let where_bounds = TokenStream2::new();

    let impl_params = {
        match *proxy_type {
            ProxyType::Ref => quote! { 'a, T: 'a },
            ProxyType::RefMut => quote! { 'a, T: 'a },
            ProxyType::Arc => quote! { T },
            ProxyType::Rc => quote! { T },
            ProxyType::Box => quote! { T },
            ProxyType::Fn => unimplemented!(),
            ProxyType::FnMut => unimplemented!(),
            ProxyType::FnOnce => unimplemented!(),
        }
    };

    quote! {
        impl< #impl_params > #trait_name for #self_ty
        where
            #where_bounds
    }
}
