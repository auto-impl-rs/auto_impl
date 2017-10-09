use std::fmt;
use syn;
use quote::Tokens;
use model::*;

pub struct WrapperTrait {
    inner: Trait,
    valid_receivers: ValidReceivers,
}

impl WrapperTrait {
    pub fn impl_rc() -> Self {
        WrapperTrait {
            inner: Trait::new("Rc", quote!(::std::rc::Rc)),
            valid_receivers: ValidReceivers {
                ref_self: true,
                no_self: true,
                ..Default::default()
            },
        }
    }

    pub fn impl_arc() -> Self {
        WrapperTrait {
            inner: Trait::new("Arc", quote!(::std::sync::Arc)),
            valid_receivers: ValidReceivers {
                ref_self: true,
                no_self: true,
                ..Default::default()
            },
        }
    }

    pub fn impl_box() -> Self {
        WrapperTrait {
            inner: Trait::new("Box", quote!(::std::boxed::Box)),
            valid_receivers: ValidReceivers {
                ref_self: true,
                ref_mut_self: true,
                value_self: true,
                no_self: true,
            },
        }
    }
}

pub struct RefTrait {
    inner: Trait,
    valid_receivers: ValidReceivers,
}

impl RefTrait {
    pub fn impl_ref() -> Self {
        RefTrait {
            inner: Trait::new("&T", quote!(&'auto)),
            valid_receivers: ValidReceivers {
                ref_self: true,
                no_self: true,
                ..Default::default()
            },
        }
    }

    pub fn impl_ref_mut() -> Self {
        RefTrait {
            inner: Trait::new("&mut T", quote!(&'auto mut)),
            valid_receivers: ValidReceivers {
                ref_self: true,
                ref_mut_self: true,
                no_self: true,
                ..Default::default()
            },
        }
    }
}

/// Auto implement a trait for a smart pointer.
/// 
/// This expects the input type to have the following properties:
/// 
/// - The smart pointer wraps a single generic value, like `Arc<T>`, `Rc<T>`
/// - The smart pointer implements `AsRef<T>`
pub fn build_wrapper(component: &AutoImpl, ref_ty: WrapperTrait) -> Result<Tokens, String> {
    let inner = ref_ty.inner;
    let impl_ident = quote!(#inner < TAutoImpl >);

    build(inner, ref_ty.valid_receivers, vec![], component, impl_ident)
}

/// Auto implement a trait for an immutable reference.
/// 
/// This expects the input to have the following properties:
/// 
/// - All methods have an `&self` receiver
pub fn build_ref(component: &AutoImpl, ref_ty: RefTrait) -> Result<Tokens, String> {
    let inner = ref_ty.inner;
    let impl_ident = quote!(#inner TAutoImpl);

    build(inner, ref_ty.valid_receivers, vec![quote!('auto)], component, impl_ident)
}

#[derive(Default)]
struct ValidReceivers {
    ref_self: bool,
    ref_mut_self: bool,
    value_self: bool,
    no_self: bool,
}

impl fmt::Display for ValidReceivers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut valid_receivers = vec![];

        if self.ref_self {
            valid_receivers.push("`&self`");
        }
        if self.ref_mut_self {
            valid_receivers.push("`&mut self`");
        }
        if self.value_self {
            valid_receivers.push("`self`");
        }
        if self.no_self {
            valid_receivers.push("static");
        }

        match valid_receivers.len() {
            0 => unreachable!(),
            1 => {
                write!(f, "{}", valid_receivers[0])
            }
            n => {
                let first = &valid_receivers[..n - 1].join(", ");
                let last = &valid_receivers[n - 1];

                write!(f, "{} or {}", first, last)
            }
        }
    }
}

fn build(ref_ty: Trait, valid_receivers: ValidReceivers, extra_lifetimes: Vec<Tokens>, component: &AutoImpl, impl_ident: Tokens) -> Result<Tokens, String> {
    let component_ident = &component.ident;

    let impl_methods = component.methods.iter()
        .map(|method| {
            let valid_receiver = match method.arg_self {
                Some(ref arg_self) => match *arg_self {
                    SelfArg::Ref(_, syn::Mutability::Immutable) => valid_receivers.ref_self,
                    SelfArg::Ref(_, syn::Mutability::Mutable) => valid_receivers.ref_mut_self,
                    SelfArg::Value(_) => valid_receivers.value_self,
                },
                None => valid_receivers.no_self
            };

            if !valid_receiver {
                Err(format!("auto impl for `{}` is only supported for methods with a {} reciever", ref_ty, valid_receivers))?
            }

            method.build_impl_item(|method| {
                let fn_ident = &method.ident;
                let fn_args = &method.arg_pats;

                match method.arg_self {
                    Some(ref arg_self) => match *arg_self {
                        // `&self` or `&mut self`
                        SelfArg::Ref(_, _) => quote!({
                            (**self).#fn_ident( #(#fn_args),* )
                        }),
                        // `self`
                        _ => quote!({
                            (*self).#fn_ident( #(#fn_args),* )
                        })
                    },
                    // No `self`
                    None => quote!({
                        TAutoImpl :: #fn_ident( #(#fn_args),* )
                    })
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

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
