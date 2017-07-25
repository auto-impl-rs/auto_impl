#![feature(proc_macro)]

extern crate quote;
extern crate proc_macro;
extern crate auto_impl_internals;

use std::str::FromStr;
use quote::Tokens;
use proc_macro::TokenStream;

fn auto_impl_expand(attrs: TokenStream, proc_tokens: TokenStream) -> Result<TokenStream, String> {
    let mut tokens = Tokens::new();
    tokens.append(proc_tokens.to_string());

    let mut attr_tokens = Tokens::new();
    attr_tokens.append(format!("#[auto_impl{}]", attrs.to_string()));

    let impl_types = derive_internals::parse_impl_types(attr_tokens)?;

    let tokens = derive_internals::auto_impl_expand(&impl_types, tokens)?;

    TokenStream::from_str(&tokens.to_string()).map_err(|e| format!("{:?}", e))
}

#[proc_macro_attribute]
pub fn auto_impl(attrs: TokenStream, proc_tokens: TokenStream) -> TokenStream {
    match auto_impl_expand(attrs, proc_tokens) {
        Ok(tokens) => tokens,
        Err(e) => panic!("auto impl error: {}", e)
    }
}
