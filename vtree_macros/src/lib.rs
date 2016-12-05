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

use syntax::parse::token::{Token, DelimToken};
use syntax::ext::base::{ExtCtxt, ProcMacro};
use syntax::symbol::keywords::Keyword;
use syntax::symbol::Symbol;
use syntax::ast::Ident;
use syntax_pos::Span;
use rustc_plugin::Registry;
use rustc_errors::DiagnosticBuilder;
use syntax::parse::parser::Parser;
use std::collections::HashMap;
use syntax::ext::base::SyntaxExtension;
use syntax::tokenstream::TokenStream;
use proc_macro_tokens::parse::lex;
use syntax::ext::quote::rt::ToTokens;

struct MyKeyword {
	#[allow(dead_code)]
	ident: Ident,
}

fn mk_keyword(s: &str) -> Keyword {
	let kw = MyKeyword {
		ident: Ident::with_empty_ctxt(Symbol::intern(s)),
	};
	unsafe {
		std::mem::transmute(kw)
	}
}

lazy_static! {
	static ref KW_MUL: Keyword = mk_keyword("mul");
	static ref KW_OPT: Keyword = mk_keyword("opt");
}

fn comma_delimiter<'a>(p: &mut Parser<'a>, t: &Token)
	-> Result<bool, DiagnosticBuilder<'a>>
{
	if p.eat(&Token::Comma) {
		if p.check(t) {
			try!(p.expect(t));
			return Ok(true);
		}
	} else {
		try!(p.expect(t));
		return Ok(true);
	}
	Ok(false)
}

#[derive(Debug)]
enum NodeChildType {
	Single,
	Optional,
	Multi,
}

#[derive(Debug)]
struct NodeChild {
	name: String,
	group: String,
	child_type: NodeChildType,
}

#[derive(Debug)]
struct Node {
	name: String,
	params_type: Option<String>,
	fields: Vec<NodeChild>,
}

fn parse_nodes<'a>(ctx: &ExtCtxt, mut p: Parser<'a>)
	-> Result<(Vec<Node>, HashMap<String, Vec<String>>), DiagnosticBuilder<'a>>
{
	let mut nodes = Vec::<Node>::new();
	let mut group_name_to_node_names = HashMap::<String, Vec<String>>::new();

	loop {
		let mut groups = Vec::<String>::new();
		let mut fields = Vec::<NodeChild>::new();
		let mut params_type = None;

		let name = try!(p.parse_ident()).name.to_string();

		if p.eat(&Token::Lt) {
			let ty = try!(p.parse_ty());
			let ts = TokenStream::from_tts(ty.to_tokens(ctx));
			params_type = Some(ts.to_string());
			try!(p.expect(&Token::Gt));
		}

		if p.eat(&Token::Colon) {
			loop {
				let group = try!(p.parse_ident());
				groups.push(group.name.to_string());

				if try!(comma_delimiter(&mut p, &Token::OpenDelim(DelimToken::Brace))) {
					break;
				}
			}
		} else {
			try!(p.expect(&Token::OpenDelim(DelimToken::Brace)));
		}

		loop {
			if p.eat(&Token::CloseDelim(DelimToken::Brace)) {
				// empty braces
				break;
			}
			let mut child_type = NodeChildType::Single;

			let field_name = try!(p.parse_ident()).name.to_string();
			try!(p.expect(&Token::Colon));

			if p.eat_keyword(*KW_OPT) {
				child_type = NodeChildType::Optional;
			} else if p.eat_keyword(*KW_MUL) {
				child_type = NodeChildType::Multi;
			}

			let field_type = try!(p.parse_ident()).name.to_string();

			fields.push(NodeChild {
				name: field_name,
				group: field_type,
				child_type: child_type,
			});

			if try!(comma_delimiter(&mut p, &Token::CloseDelim(DelimToken::Brace))) {
				break;
			}
		}

		nodes.push(Node {
			name: name.clone(),
			params_type: params_type,
			fields: fields,
		});

		for group in groups {
			{
				if let Some(nodes) = group_name_to_node_names.get_mut(&group) {
					nodes.push(name.clone());
					continue;
				}
			}
			group_name_to_node_names.insert(group, vec![name.clone()]);
		}

		if try!(comma_delimiter(&mut p, &Token::Eof)) {
			break;
		}
	}

	Ok((nodes, group_name_to_node_names))
}

fn to_snake_case(s: &str) -> String {
	use regex::{Regex, Captures};
	lazy_static! {
		static ref RE: Regex = Regex::new("([a-zA-Z]|^)([A-Z])").unwrap();
	}
	RE.replace_all(s, |caps: &Captures| {
		let cap1 = caps.at(1).unwrap_or("");
		let cap2 = caps.at(2).unwrap_or("").to_lowercase();
		if cap1.len() != 0 {
			format!("{}_{}", cap1, cap2)
		} else {
			cap2
		}
	})
}

fn to_ident(s: &str) -> quote::Ident {
	use quote::Ident;
	Ident::from(s)
}

fn generate_defs(nodes: Vec<Node>, group_name_to_node_names: HashMap<String, Vec<String>>) -> TokenStream {
	let node_defs = nodes.iter().map(|node| {
		let fields = node.fields.iter().map(|field| {
			let name = to_ident(&field.name);
			let group = to_ident(&field.group);
			match field.child_type {
				NodeChildType::Single => quote!{
					pub #name: ::std::boxed::Box<#group>
				},
				NodeChildType::Optional => quote!{
					pub #name: ::std::option::Option<::std::boxed::Box<#group>>
				},
				NodeChildType::Multi => quote!{
					pub #name: ::vtree::key::KeyedNodes<#group>
				},
			}
		});

		let params_field = node.params_type.as_ref().map(|params| {
			let params = to_ident(params);
			quote!{
				pub params: #params
			}
		});

		let name = to_ident(&node.name);
		quote!{
			#[derive(Debug, Clone)]
			pub struct #name {
				#params_field,
				#(#fields,)*
			}
		}
	});

	let group_defs = group_name_to_node_names.iter().map(|(group, nodes)| {
		let vars = nodes.iter().map(|node| {
			let node = to_ident(node);
			quote!{
				#node(#node)
			}
		});

		let name = to_ident(&group);
		quote!{
			#[derive(Debug, Clone)]
			pub enum #name {
				Widget(::std::boxed::Box<::vtree::widget::WidgetDataTrait<#name>>),
				#(#vars,)*
			}
		}
	});

	let differ_element_diff_groups = group_name_to_node_names.keys().map(|group| {
		let name_diff_fn = quote::Ident::from(format!("diff_{}", to_snake_case(group)));
		let name_group = to_ident(group);
		quote!{
			fn #name_diff_fn<'a>(
				&self,
				path: &::vtree::diff::Path,
				curr: &#name_group,
				diff: ::vtree::diff::Diff,
			);
		}
	});
	let differ_element_reorders = nodes.iter().flat_map(|node| {
		let name_node_sc = to_snake_case(&node.name);
		node.fields.iter().map(move |field| {
			let name_fn = quote::Ident::from(format!("reorder_{}_{}",
				name_node_sc,
				to_snake_case(&field.name)
			));
			quote!{
				fn #name_fn(
					&self,
					path: &::vtree::diff::Path,
					index_curr: usize,
					index_last: usize,
				);
			}
		})
	});
	let differ_element_params_changed_nodes = nodes.iter()
		.filter(|node| node.params_type.is_some())
		.map(|node| {
			let name_node = to_ident(&node.name);
			let name_fn = quote::Ident::from(format!("params_changed_{}", to_snake_case(&node.name)));
			quote!{
				fn #name_fn(
					&self,
					path: &::vtree::diff::Path,
					curr: &#name_node,
					last: &#name_node,
				);
			}
		});
	let differ_elements = differ_element_diff_groups
		.chain(differ_element_reorders)
		.chain(differ_element_params_changed_nodes);

	let defs = quote!{
		#(#node_defs)*

		#(#group_defs)*

		pub trait Differ {
			#(#differ_elements)*
		}
	};
	println!("{}", defs);
	lex(defs.as_str())
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
