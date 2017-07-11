#![feature(conservative_impl_trait)]
#![feature(proc_macro)]
#![recursion_limit = "128"]

#[macro_use]
extern crate quote;
extern crate syn;
#[macro_use]
extern crate synom;
extern crate proc_macro;

mod parser;
mod generator;
mod params;

use parser::parse;
use generator::generate_defs;
use proc_macro::TokenStream;
use params::handle_params;

#[proc_macro]
pub fn define_nodes(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let pd = parse(&input);
    for param_ty in pd.normal_nodes().filter_map(|node| node.params_ty.as_ref()) {
        assert!(param_ty.global, "`{}` is not a global module Path", quote!{#param_ty});
    }
    generate_defs(pd).parse().unwrap()
}

#[proc_macro]
pub fn define_params(input: TokenStream) -> TokenStream {
    let res = handle_params(input.to_string());
    println!("{}\n", res);
    res.parse().unwrap()
}
