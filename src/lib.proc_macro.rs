extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn auto_impl(attrs: TokenStream, proc_tokens: TokenStream) -> TokenStream {
    fn expand(attrs: TokenStream, proc_tokens: TokenStream) -> Result<TokenStream, String> {
        let mut tokens = Tokens::new();
        tokens.append(proc_tokens.to_string());

        let mut attr_tokens = Tokens::new();
        attr_tokens.append(format!("#[auto_impl{}]", attrs.to_string()));

        let impl_types = parse_impl_types(attr_tokens)?;

        let tokens = auto_impl_expand(&impl_types, tokens)?;

        TokenStream::from_str(&tokens.to_string()).map_err(|e| format!("{:?}", e))
    }

    match expand(attrs, proc_tokens) {
        Ok(tokens) => tokens,
        Err(e) => panic!("auto impl error: {}", e)
    }
}
