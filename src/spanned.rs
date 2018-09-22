use crate::proc_macro::{Span, TokenStream};
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;


/// Helper trait to receive the span of a `syn` AST node. This is very similar
/// to `syn::Spanned`, but differs in that it results in a `proc_macro::Span`
/// and not a `proc_macro2::Span`. This is way better since we can directly
/// emit errors and warning from the former ones.
///
/// This trait is implemented for all types that implement `ToTokens`. The
/// implementation is rather ugly, since we generate a complete token stream
/// of the node to get the span information of the underlying tokens.
pub trait Spanned {
    /// Returns the span of this value. The value is expected not to span
    /// multiple files -- else this method panics.
    fn span(&self) -> Span;
}

impl<T: ToTokens> Spanned for T {
    fn span(&self) -> Span {
        // Convert the node into tokens
        let mut tokens = TokenStream2::new();
        self.to_tokens(&mut tokens);
        let tokens: TokenStream = tokens.into();

        if tokens.is_empty() {
            Span::call_site()
        } else {
            // If we're on nightly, we can create a correct span. Otherwise we
            // just point to the first token.
            #[cfg(feature = "nightly")]
            let span = {
                let mut iter = tokens.into_iter();
                let mut span = iter.next().unwrap().span();
                if let Some(last) = iter.last() {
                    span = span.join(last.span()).unwrap();
                }
                span
            };

            #[cfg(not(feature = "nightly"))]
            let span = tokens.into_iter().next().unwrap().span();

            span
        }
    }
}
