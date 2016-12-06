#![feature(plugin_registrar, rustc_private)]

#[macro_use]
extern crate quote;
extern crate syntax;
extern crate syntax_pos;
extern crate rustc_plugin;
extern crate rustc_errors;
#[macro_use]
extern crate lazy_static;
extern crate proc_macro_tokens;
extern crate regex;

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

#[derive(Debug)]
pub enum NodeChildType {
	Single,
	Optional,
	Multi,
}

#[derive(Debug)]
pub struct NodeChild {
	name: String,
	group: String,
	child_type: NodeChildType,
}

#[derive(Debug)]
pub struct Node {
	name: String,
	params_type: Option<String>,
	fields: Vec<NodeChild>,
}

struct MacroDefineNodes;
impl ProcMacro for MacroDefineNodes
{
	fn expand<'ctx>(&self, ctx: &'ctx mut ExtCtxt, _span: Span, ts: TokenStream) -> TokenStream {
		let tts = ts.to_tts();
		let (nodes, group_name_to_node_names) = parse_nodes(ctx, ctx.new_parser_from_tts(&tts)).unwrap();
		generate_defs(nodes, group_name_to_node_names)
	}
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
	reg.register_syntax_extension(
		Symbol::intern("define_nodes"),
		SyntaxExtension::ProcMacro(Box::new(MacroDefineNodes))
	);
}
