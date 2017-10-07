use syn;
use quote::Tokens;
use model::*;

/// Auto implement a trait for a smart pointer.
/// 
/// This expects the input type to have the following properties:
/// 
/// - The smart pointer wraps a single generic value, like `Arc<T>`, `Box<T>`, `Rc<T>`
/// - The smart pointer implements `AsRef<T>`
pub fn build_wrapper(component: &AutoImpl, ref_ty: Trait) -> Result<Tokens, String> {
    let impl_methods = component.methods.iter()
        .map(|method| {
            let valid_receiver = match method.arg_self {
                Some(ref arg_self) => match *arg_self {
                    SelfArg::Ref(_, syn::Mutability::Immutable) => true,
                    _ => false
                },
                None => false
            };

            if !valid_receiver {
                Err(format!("auto impl for `{}` is only supported for methods with a `&self` reciever", ref_ty))?
            }

            method.build_impl_item(|method| {
                let fn_ident = &method.ident;
                let fn_args = &method.arg_pats;

                quote!({
                    self.as_ref().#fn_ident( #(#fn_args),* )
                })
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    build(component, vec![], quote!(#ref_ty < TAutoImpl >), impl_methods)
}

/// Auto implement a trait for an immutable reference.
/// 
/// This expects the input to have the following properties:
/// 
/// - All methods have an `&self` receiver
pub fn build_immutable(component: &AutoImpl) -> Result<Tokens, String> {
    let impl_methods = component.methods.iter()
        .map(|method| {
            let valid_receiver = match method.arg_self {
                Some(ref arg_self) => match *arg_self {
                    SelfArg::Ref(_, syn::Mutability::Immutable) => true,
                    _ => false
                },
                None => false
            };

            if !valid_receiver {
                Err("auto impl for `&T` is only supported for methods with a `&self` reciever")?
            }

            method.build_impl_item(|method| {
                let fn_ident = &method.ident;
                let fn_args = &method.arg_pats;

                quote!({
                    (**self).#fn_ident( #(#fn_args),* )
                })
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    build(component, vec![quote!('auto)], quote!(&'auto TAutoImpl), impl_methods)
}

/// Auto implement a trait for a mutable reference.
pub fn build_mutable(component: &AutoImpl) -> Result<Tokens, String> {
    let impl_methods = component.methods.iter()
        .map(|method| {
            method.build_impl_item(|method| {
                let fn_ident = &method.ident;
                let fn_args = &method.arg_pats;

                quote!({
                    (**self).#fn_ident( #(#fn_args),* )
                })
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    
    build(component, vec![quote!('auto)], quote!(&'auto mut TAutoImpl), impl_methods)
}

fn build(component: &AutoImpl, extra_lifetimes: Vec<Tokens>, impl_ident: Tokens, impl_methods: Vec<syn::TraitItem>) -> Result<Tokens, String> {
    let component_ident = &component.ident;

    let impl_associated_types = component.associated_types.iter()
        .map(|associated_type| {
            associated_type.build_impl_item(|associated_type| {
                let ty_ident = &associated_type.ident;

                quote!(TAutoImpl :: #ty_ident)
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let (trait_tys, impl_lifetimes, impl_tys, where_clauses) = component.split_generics();

    Ok(quote!(
        impl< #(#extra_lifetimes,)* #(#impl_lifetimes,)* #(#impl_tys,)* TAutoImpl > #component_ident #trait_tys for #impl_ident
            where TAutoImpl: #component_ident #trait_tys
                  #(,#where_clauses)*
        {
            #(#impl_associated_types)*

            #(#impl_methods)*
        }
    ))
}
