//! A proc-macro attribute for automatically implementing a trait for
//! references, some common smart pointers and closures.


#![cfg_attr(feature = "nightly", feature(proc_macro_diagnostic, proc_macro_span))]

extern crate proc_macro;
#[macro_use]
extern crate quote;

use crate::proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;

mod analyze;
mod attr;
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
    // Try to parse the token stream from the attribute to get a list of proxy
    // types.
    let proxy_types = proxy::parse_types(args);

    // Try to parse the item the `#[auto_impl]` attribute was applied to as
    // trait. Unfortunately, `parse()` consume the token stream, so we have to
    // clone it.
    match syn::parse::<syn::ItemTrait>(input.clone()) {
        // The attribute was applied to a valid trait. Now it's time to execute
        // the main step: generate a token stream which contains an impl of the
        // trait for each proxy type.
        Ok(mut trait_def) => {
            let generated = proxy_types.and_then(|proxy_types| {
                gen::gen_impls(&proxy_types, &trait_def)
            });

            // Before returning the trait definition, we have to remove all
            // `#[auto_impl(...)]` attributes on all methods.
            attr::remove_our_attrs(&mut trait_def);

            match generated {
                // No errors at all => output the trait and our generated impls
                Ok(generated) => quote! { #trait_def #generated }.into(),
                Err(_) => {
                    // We combine the token stream of the modified trait
                    // definition with the generated errors (which are tokens
                    // on stable due to the `compile_error!` hack).
                    vec![
                        TokenStream::from(trait_def.into_token_stream()),
                        diag::error_tokens()
                    ].into_iter().collect()
                }
            }
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

            // We combine the original token stream with the generated errors
            // (which are tokens on stable due to the `compile_error!` hack).
            vec![input, diag::error_tokens()].into_iter().collect()
        }
    }
}
