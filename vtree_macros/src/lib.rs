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

use parser::parse;
use generator::generate_defs;
use proc_macro::TokenStream;

#[proc_macro]
pub fn define_nodes(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let pd = parse(&input);
    println!("{:?}", pd);
    generate_defs(pd).parse().unwrap()
}
