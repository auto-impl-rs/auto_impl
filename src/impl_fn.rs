use syn;
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
/// - It has no associated type
/// - It has no non-static lifetimes in the return type
pub fn build(component: &AutoImpl, ref_ty: Trait) -> Result<Tokens, String> {
    let method = expect_single_method(component, &ref_ty)?;

    expect_static_lifetimes_in_return_ty(&method, &ref_ty)?;
    
    if component.associated_types.len() > 0 {
        Err(format!("auto impl for `{}` is not supported for associated types", ref_ty))?
    }

    let component_ident = &component.ident;

    let fn_arg_tys = method.anonymous_arg_lifetimes();
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

fn expect_static_lifetimes_in_return_ty(method: &AutoImplMethod, ref_ty: &Trait) -> Result<(), String> {
    fn is_static(lifetime: &syn::Lifetime) -> bool {
        lifetime.ident.as_ref() == "'static"
    }

    fn only_static_lifetimes(ty: &syn::Ty) -> bool {
        match *ty {
            syn::Ty::Slice(ref ty) => only_static_lifetimes(&ty),
            syn::Ty::Array(ref ty, _) => only_static_lifetimes(&ty),
            syn::Ty::Ptr(ref mut_ty) => only_static_lifetimes(&mut_ty.ty),
            syn::Ty::Tup(ref tys) => tys.iter().all(only_static_lifetimes),
            syn::Ty::Paren(ref ty) => only_static_lifetimes(&ty),
            syn::Ty::Path(_, ref path) => {
                path.segments.iter().all(|segment| {
                    match segment.parameters {
                        syn::PathParameters::AngleBracketed(ref params) => {
                            params.lifetimes.iter().all(is_static) && params.types.iter().all(only_static_lifetimes)
                        },
                        syn::PathParameters::Parenthesized(ref params) => {
                            let output_is_static = match params.output {
                                Some(ref ty) => only_static_lifetimes(ty),
                                _ => true
                            };

                            params.inputs.iter().all(only_static_lifetimes) && output_is_static
                        }
                    }
                })
            },
            syn::Ty::Rptr(ref lifetime, ref mut_ty) => {
                let is_static = match lifetime {
                    &Some(ref l) => is_static(l),
                    _ => false
                };

                is_static && only_static_lifetimes(&mut_ty.ty)
            },
            _ => true
        }
    }

    if let Some(ref ty) = method.output {
        if !only_static_lifetimes(ty) {
            Err(format!("auto impl for `{}` is not supported for non-static lifetimes in return types", ref_ty))?
        }
    }

    Ok(())
}

fn expect_single_method<'a>(component: &'a AutoImpl, ref_ty: &Trait) -> Result<&'a AutoImplMethod, String> {
    if component.methods.len() != 1 {
        Err(format!("auto impl for `{}` is only supported for traits with 1 method", ref_ty))?
    }

    let method = component.methods.iter().next().expect("");

    Ok(method)
}