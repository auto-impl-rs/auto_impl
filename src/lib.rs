#![feature(crate_in_paths)]
#![feature(extern_prelude)]
#![feature(proc_macro)]


extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
extern crate syn;



use proc_macro::{
    TokenStream, Diagnostic, Level, Span,
};


mod gen;
mod proxy;


#[proc_macro_attribute]
pub fn auto_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    // We use the closure trick to catch errors until the `catch` syntax is
    // available. If an error occurs, we won't modify or add any tokens.
    let impls = || -> Result<TokenStream, ()> {
        // Try to parse the token stream from the attribute to get a list of
        // proxy types.
        let proxy_types = proxy::parse_types(args)?;
        println!("Proxy types: {:?}", proxy_types);

        // Try to parse the
        match syn::parse::<syn::ItemTrait>(input.clone()) {
            Ok(trait_def) => {
                // println!("{:#?}", trait_def);

                let impls = gen::gen_impls(&proxy_types, &trait_def)?;
                Ok(impls)
            }
            Err(e) => {
                let msg = "couldn't parse trait item";
                Diagnostic::spanned(Span::call_site(), Level::Error, msg)
                    .note(e.to_string())
                    .note("the #[auto_impl] attribute can only be applied to traits!")
                    .emit();

                Err(())
            }
        }
    }().unwrap_or(TokenStream::new());

    // Combine the original token stream with the additional one containing the
    // generated impls (or nothing if an error occured).
    let out = vec![input, impls].into_iter().collect();

    // println!("--------------------");
    // println!("{}", out);
    // println!("--------------------");

    out
}
