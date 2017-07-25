use syn;
use quote::Tokens;
use model::*;

/// Auto implement a trait for a smart pointer.
/// 
/// This expects the input type to have the following properties:
/// 
/// - The smart pointer wraps a single generic value, like `Arc<T>`, `Box<T>`, `Rc<T>`
/// - The smart pointer implements `AsRef<T>`
pub fn build(component: &AutoImpl, ref_ty: Trait) -> Result<Tokens, String> {
    let component_ident = &component.ident;

    let impl_methods = try_iter!(
        component.methods.iter()
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
    );

    let impl_associated_types = try_iter!(
        component.associated_types.iter()
            .map(|associated_type| {
                associated_type.build_impl_item(|associated_type| {
                    let ty_ident = &associated_type.ident;

                    quote!(TAutoImpl :: #ty_ident)
                })
            })
    );

    let (trait_tys, impl_lifetimes, impl_tys, where_clauses) = component.split_generics();

    Ok(quote!(
        impl< #(#impl_lifetimes,)* #(#impl_tys,)* TAutoImpl > #component_ident #trait_tys for #ref_ty < TAutoImpl >
            where TAutoImpl: #component_ident #trait_tys
                  #(,#where_clauses)*
        {
            #(#impl_associated_types)*

            #(#impl_methods)*
        }
    ))
}
