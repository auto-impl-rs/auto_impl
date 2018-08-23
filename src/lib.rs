//! A proc-macro attribute for automatically implementing a trait for
//! references, some common smart pointers and closures.


#![cfg_attr(feature = "nightly", feature(proc_macro_diagnostic, proc_macro_span))]

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{
    TraitItemMethod,
    visit_mut::{VisitMut, visit_item_trait_mut},
};

mod analyze;
mod diag;
mod gen;
mod proxy;
mod spanned;

use crate::{
    diag::SpanExt,
    spanned::Spanned,
};


/// See crate documentation for more information.
#[proc_macro_attribute]
pub fn auto_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    // We use the closure trick to catch errors until the `catch` syntax is
    // available. If an error occurs, we won't modify or add any tokens.
    let result = || -> Result<TokenStream, ()> {
        // Try to parse the token stream from the attribute to get a list of
        // proxy types.
        let proxy_types = proxy::parse_types(args)?;

        // Try to parse the item the `#[auto_impl]` attribute was applied to as
        // trait. Unfortunately, `parse()` consume the token stream, so we have
        // to clone it.
        match syn::parse::<syn::ItemTrait>(input.clone()) {
            // The attribute was applied to a valid trait. Now it's time to
            // execute the main step: generate a token stream which contains an
            // impl of the trait for each proxy type.
            Ok(mut trait_def) => {
                let generated = gen::gen_impls(&proxy_types, &trait_def)?;

                // Before returning the trait definition, we have to remove all
                // `#[auto_impl(...)]` attributes on any methods.
                remove_our_attrs(&mut trait_def);

                Ok(quote! { #trait_def #generated }.into())
            },

            // If the token stream could not be parsed as trait, this most
            // likely means that the attribute was applied to a non-trait item.
            // Even if the trait definition was syntactically incorrect, the
            // compiler usually does some kind of error recovery to proceed. We
            // get the recovered tokens.
            Err(e) => {
                // We have to take the detour through TokenStream2 to get a
                // good span for the error.
                TokenStream2::from(input.clone()).span()
                    .err("couldn't parse trait item")
                    .note(e.to_string())
                    .note("the #[auto_impl] attribute can only be applied to traits!")
                    .emit();

                Err(())
            }
        }
    }();

    // If everything went well, we just return the new token stream. If an
    // error occured, we combine the original token stream with the generated
    // errors (which are tokens on stable due to the `compile_error!` hack).
    match result {
        Ok(tokens) => tokens,
        Err(_) => vec![input, diag::error_tokens()].into_iter().collect(),
    }
}

/// Removes all `#[auto_impl]` attributes that are attached to methods of the
/// given trait.
fn remove_our_attrs(trait_def: &mut syn::ItemTrait) {
    struct AttrRemover;
    impl VisitMut for AttrRemover {
        fn visit_trait_item_method_mut(&mut self, m: &mut TraitItemMethod) {
            m.attrs.retain(|a| !a.path.segments.iter().all(|seg| {
                seg.ident == "auto_impl" && seg.arguments.is_empty()
            }));
        }
    }

    visit_item_trait_mut(&mut AttrRemover, trait_def);
}
