use proc_macro2::{
    TokenStream as TokenStream2,
    Span,
};
use quote::{TokenStreamExt, ToTokens};
use syn::{
    GenericParam, TypeParam, LifetimeDef, ItemImpl, Path, PathSegment, PathArguments, Type,
    TypeReference, TypePath, Ident, Lifetime, Generics, TypeParamBound,
    punctuated::Punctuated,
    token,
};


use crate::proxy::ProxyType;

const PROXY_TY_PARAM_NAME: &str = "__AutoImplProxyT";
const PROXY_LT_PARAM_NAME: &str = "'__auto_impl_proxy_lifetime";

// fn proxy_ty_param_ident() -> TokenStream2 {
//     quote! { __ProxyT }
// }

// fn proxy_lifetime_ident() -> TokenStream2 {
//     quote! { '__proxy_lifetime }
// }

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

// pub(crate) fn gen_impls(
//     proxy_types: &[ProxyType],
//     trait_def: &syn::ItemTrait,
// ) -> Result<::proc_macro::TokenStream, ()> {
//     let mut tokens = TokenStream2::new();

//     // One impl for each proxy type
//     for proxy_type in proxy_types {
//         impl_def(proxy_type, trait_def)?.to_tokens(&mut tokens);
//     }

//     Ok(tokens.into())
// }

// fn impl_def(proxy_type: &ProxyType, trait_def: &syn::ItemTrait) -> Result<ItemImpl, ()> {
//     let (impl_generics, ty_generics, _where_clause) = trait_def.generics.split_for_impl();

//     let trait_ = {
//         let mut segments = Punctuated::new();
//         segments.push(PathSegment {
//             ident: trait_def.ident.clone(),
//             arguments: PathArguments::AngleBracketed(, // TODO
//         });

//         let path = Path {
//             leading_colon: None,
//             segments,
//         };

//         // No bang, trait name, simple `for`
//         (None, path, Token![for](Span::call_site()))
//     };

//     let impl_generics = {
//         // The only way to get normal `Generics` from `ImplGenerics` is to
//         // first convert it into a token stream and then parse it.
//         let mut g = syn::parse2::<Generics>(impl_generics.into_token_stream()).unwrap();

//         let lifetime = Lifetime::new(PROXY_LT_PARAM_NAME, Span::call_site());

//         if proxy_type.is_ref() {
//             let lifetime_def = LifetimeDef {
//                 attrs: vec![],
//                 lifetime: lifetime.clone(),
//                 colon_token: None,
//                 bounds: Punctuated::new(),
//             };
//             g.params.insert(0, GenericParam::Lifetime(lifetime_def));
//         }

//         let bounds = if proxy_type.is_ref() {
//             let mut b = Punctuated::new();
//             b.push(TypeParamBound::Lifetime(lifetime));
//             b
//         } else {
//             Punctuated::new()
//         };

//         g.params.push(GenericParam::Type(TypeParam {
//             attrs: vec![],
//             ident: Ident::new(PROXY_TY_PARAM_NAME, Span::call_site()),
//             colon_token: Some(Token![:]([Span::call_site()])),
//             bounds,
//             eq_token: None,
//             default: None,
//         }));

//         g
//     };

//     let item = ItemImpl {
//         attrs: vec![],
//         defaultness: None,
//         unsafety: None,
//         impl_token: Token![impl](Span::call_site()),
//         generics: impl_generics, //Generics,
//         trait_: Some(trait_),
//         self_ty: Box::new(self_type(proxy_type)),
//         brace_token: token::Brace(Span::call_site()),
//         items: vec![],
//     };

//     Ok(item)
// }

// fn self_type(proxy_type: &ProxyType) -> Type {
//     let type_t = {
//         let mut segments = Punctuated::new();
//         segments.push(PathSegment {
//             ident: Ident::new(PROXY_TY_PARAM_NAME, Span::call_site()),
//             arguments: PathArguments::None, // TODO
//         });

//         Type::Path(TypePath {
//             qself: None,
//             path: Path {
//                 leading_colon: None,
//                 segments,
//             },
//         })
//     };


//     match *proxy_type {
//         ProxyType::Ref => Type::Reference(TypeReference {
//             and_token: Token!(&)([Span::call_site()]),
//             lifetime: Some(Lifetime::new(PROXY_LT_PARAM_NAME, Span::call_site())),
//             mutability: None,
//             elem: Box::new(type_t),
//         }),
//         ProxyType::RefMut => Type::Reference(TypeReference {
//             and_token: Token![&]([Span::call_site()]),
//             lifetime: Some(Lifetime::new(PROXY_LT_PARAM_NAME, Span::call_site())),
//             mutability: Some(Token![mut](Span::call_site())),
//             elem: Box::new(type_t),
//         }),
//         // ProxyType::Arc      => ,
//         // ProxyType::Rc       => ,
//         // ProxyType::Box      => ,
//         ProxyType::Fn => type_t,
//         ProxyType::FnMut => type_t,
//         ProxyType::FnOnce => type_t,
//         _ => unimplemented!(),
//     }
// }

fn header(proxy_type: &ProxyType, trait_def: &syn::ItemTrait) -> TokenStream2 {
    let proxy_ty_param = Ident::new(PROXY_TY_PARAM_NAME, Span::call_site()); // proxy_ty_param_ident();

    // The tokens after `for` in the impl header
    let self_ty = match *proxy_type {
        ProxyType::Ref      => quote! { &'a #proxy_ty_param },
        ProxyType::RefMut   => quote! { &'a mut #proxy_ty_param },
        ProxyType::Arc      => quote! { ::std::sync::Arc<#proxy_ty_param> },
        ProxyType::Rc       => quote! { ::std::rc::Rc<#proxy_ty_param> },
        ProxyType::Box      => quote! { ::std::boxed::Box<#proxy_ty_param> },
        ProxyType::Fn       => quote! { proxy_ty_param },
        ProxyType::FnMut    => quote! { proxy_ty_param },
        ProxyType::FnOnce   => quote! { proxy_ty_param },
    };

    let (impl_generics, trait_generics, where_clause) = trait_def.generics.split_for_impl();


    // Here we assemble the parameter list of the impl (the thing in
    // `impl< ... >`). This is simply the parameter list of the trait with
    // one or two parameters added. For a trait `trait Foo<'x, 'y, A, B>`,
    // it will look like this:
    //
    //    '__auto_impl_proxy_lifetime, 'x, 'y, A, B, __AutoImplProxyT
    //
    // The `'__auto_impl_proxy_lifetime` in the beginning is only added when
    // the proxy type is `&` or `&mut`.
    let impl_params = {
        // Determine if our proxy type needs a lifetime parameter
        let (mut params, ty_bounds) = match proxy_type {
            ProxyType::Ref | ProxyType::RefMut => {
                let lifetime = &Lifetime::new(PROXY_LT_PARAM_NAME, Span::call_site());
                (quote! { #lifetime, }, quote! { : #lifetime })
            }
            ProxyType::Box | ProxyType::Rc | ProxyType::Arc => {
                (quote! {}, quote! {})
            }
            ProxyType::Fn       => (quote! {}, quote! { : ::std::ops::Fn }),
            ProxyType::FnMut    => (quote! {}, quote! { : ::std::ops::FnMut }),
            ProxyType::FnOnce   => (quote! {}, quote! { : ::std::ops::FnOnce }),
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

        // Append proxy type parameter
        params.append_all(quote! { , #proxy_ty_param #ty_bounds });

        params
    };

    let trait_name = &trait_def.ident;

    quote! {
        impl<#impl_params> #trait_name #trait_generics for #self_ty
        where
            #where_clause
    }
}
