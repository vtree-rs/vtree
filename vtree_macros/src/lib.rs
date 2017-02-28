#![feature(plugin_registrar, rustc_private)]
#![feature(conservative_impl_trait)]
#![recursion_limit = "128"]

#[macro_use]
extern crate quote;
extern crate syntax;
extern crate syntax_pos;
extern crate rustc_plugin;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate syn;
#[macro_use]
extern crate synom;

mod parser;
mod generator;

use parser::parse_nodes;
use generator::generate_defs;

use syntax::ext::base::{ExtCtxt, ProcMacro};
use syntax::symbol::Symbol;
use syntax_pos::Span;
use rustc_plugin::Registry;
use syntax::ext::base::SyntaxExtension;
use syntax::tokenstream::TokenStream;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum NodeChildType {
    Single,
    Optional,
    Multi,
}

#[derive(Debug, Clone)]
pub struct NodeChild {
    name: syn::Ident,
    group: syn::Ident,
    child_type: NodeChildType,
}

#[derive(Debug, Clone)]
pub struct Node {
    name: syn::Ident,
    params_type: Option<syn::Path>,
    fields: Vec<NodeChild>,
}

#[derive(Debug, Clone)]
pub struct ParsedData {
    nodes: Vec<Node>,
    group_name_to_nodes: HashMap<syn::Ident, Vec<Node>>,
}

struct MacroDefineNodes;
impl ProcMacro for MacroDefineNodes {
    fn expand<'ctx>(&self, _: &'ctx mut ExtCtxt, _: Span, ts: TokenStream) -> TokenStream {
        let input = ts.to_string();
        let pd = parse_nodes(&input);
        generate_defs(pd)
    }
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_syntax_extension(Symbol::intern("define_nodes"),
                                  SyntaxExtension::ProcMacro(Box::new(MacroDefineNodes)));
}
