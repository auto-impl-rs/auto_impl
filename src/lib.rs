#![feature(proc_macro)]


// extern crate proc_macro2;
extern crate proc_macro;
// #[macro_use]
extern crate quote;
extern crate syn;



use proc_macro::{
    TokenStream,
};


mod proxy;


#[proc_macro_attribute]
pub fn auto_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let proxy_types = proxy::parse_types(args);
    println!("{:?}", proxy_types);

    input
}
