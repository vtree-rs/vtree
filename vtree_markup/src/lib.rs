#![feature(proc_macro)]

#[macro_use]
extern crate quote;
extern crate syn;
#[macro_use]
extern crate synom;
extern crate proc_macro;

mod parser;
mod generator;

use parser::parse_node;
use proc_macro::TokenStream;

#[proc_macro]
pub fn markup(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let output = parse_node(&input).expect("vtree markup");
    println!("{:?}", output);
    "".parse().unwrap()
}
