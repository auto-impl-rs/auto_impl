//! A proc-macro attribute for automatically implementing a trait for
//! references, some common smart pointers and closures.


#![cfg_attr(feature = "nightly", feature(proc_macro_diagnostic, proc_macro_span))]

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

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
    let impls = || -> Result<TokenStream, ()> {
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
            Ok(trait_def) => Ok(gen::gen_impls(&proxy_types, &trait_def)?),

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
    }().unwrap_or_else(|_| diag::error_tokens());

    // Combine the original token stream with the additional one containing the
    // generated impls (or nothing if an error occured).
    vec![input, impls].into_iter().collect()
}
