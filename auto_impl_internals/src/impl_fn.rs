use quote::Tokens;
use model::*;

/// Auto implement a trait for a function.
/// 
/// This expects the input type to have the following properties:
/// 
/// - It is a function trait that supports the `Fn(input) -> output` syntax sugar
/// 
/// This expects the input to have the following properties:
/// 
/// - It has a single method
pub fn build(component: &AutoImpl, ref_ty: Trait) -> Result<Tokens, String> {
    let method = expect_single_method(component, &ref_ty)?;
    
    if component.associated_types.len() > 0 {
        Err(format!("auto impl for `{}` is not supported for associated types", ref_ty))?
    }

    let component_ident = &component.ident;

    let fn_arg_tys = &method.arg_tys;
    let fn_output = &method.output;

    let impl_method = method
        .build_impl_item(|method| {
            let fn_args = &method.arg_pats;

            quote!({
                self( #(#fn_args),* )
            })
        })?;

    let return_ty = method.output.clone().map(|_| {
        quote!(-> #fn_output)
    });

    let (trait_tys, impl_lifetimes, impl_tys, where_clauses) = component.split_generics();

    Ok(quote!(
        impl< #(#impl_lifetimes,)* #(#impl_tys,)* TFn> #component_ident #trait_tys for TFn
            where TFn: #ref_ty ( #(#fn_arg_tys),* ) #return_ty
                  #(,#where_clauses)*
        {
            #impl_method
        }
    ))
}

fn expect_single_method<'a>(component: &'a AutoImpl, ref_ty: &Trait) -> Result<&'a AutoImplMethod, String> {
    if component.methods.len() != 1 {
        Err(format!("auto impl for `{}` is only supported for traits with 1 method", ref_ty))?
    }

    let method = component.methods.iter().next().expect("");

    Ok(method)
}