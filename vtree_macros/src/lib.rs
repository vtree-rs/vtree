#![feature(plugin, plugin_registrar, rustc_private)]
#![plugin(quasi_macros)]
#![feature(slice_patterns)]

extern crate quasi;
extern crate syntax;
extern crate syntax_pos;
extern crate rustc;
extern crate rustc_plugin;
extern crate rustc_errors;
#[macro_use]
extern crate lazy_static;
extern crate aster;

use syntax::parse::token::{Token, DelimToken};
use syntax::tokenstream::TokenTree;
use syntax::ext::base::{ExtCtxt, MacEager, MacResult, DummyResult};
use syntax::symbol::keywords::{self, Keyword};
use syntax::symbol::Symbol;
use syntax::ast::{Name, Ident, Ty};
use syntax::ptr::P;
use syntax::util::small_vector::SmallVector;
use syntax_pos::Span;
use rustc_plugin::Registry;
use rustc_errors::DiagnosticBuilder;
use syntax::parse::parser::Parser;
use std::collections::HashMap;
use aster::ident::ToIdent;

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
	name: Name,
	group: Name,
	is_public: bool,
	child_type: NodeChildType,
}

#[derive(Debug)]
struct Node {
	name: Name,
	params_type: Option<P<Ty>>,
	fields: Vec<NodeChild>,
}

fn parse_nodes<'a>(mut p: Parser<'a>)
	-> Result<(Vec<Node>, HashMap<Name, Vec<Name>>), DiagnosticBuilder<'a>>
{
	let mut nodes = Vec::<Node>::new();
	let mut group_name_to_node_names = HashMap::<Name, Vec<Name>>::new();

	loop {
		let mut groups = Vec::<Name>::new();
		let mut fields = Vec::<NodeChild>::new();
		let mut params_type = None;

		let name = try!(p.parse_ident()).name;

		if p.eat(&Token::Lt) {
			params_type = Some(try!(p.parse_ty()));
			try!(p.expect(&Token::Gt));
		}

		if p.eat(&Token::Colon) {
			loop {
				let group = try!(p.parse_ident());
				groups.push(group.name);

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

			let is_public = p.eat_keyword(keywords::Pub);

			let field_name = try!(p.parse_ident()).name;
			try!(p.expect(&Token::Colon));

			if p.eat_keyword(*KW_OPT) {
				child_type = NodeChildType::Optional;
			} else if p.eat_keyword(*KW_MUL) {
				child_type = NodeChildType::Multi;
			}

			let field_type = try!(p.parse_ident()).name;

			fields.push(NodeChild {
				name: field_name,
				group: field_type,
				is_public: is_public,
				child_type: child_type,
			});

			if try!(comma_delimiter(&mut p, &Token::CloseDelim(DelimToken::Brace))) {
				break;
			}
		}

		nodes.push(Node {
			name: name,
			params_type: params_type,
			fields: fields,
		});

		for group in groups {
			{
				if let Some(nodes) = group_name_to_node_names.get_mut(&group) {
					nodes.push(name);
					continue;
				}
			}
			group_name_to_node_names.insert(group, vec![name]);
		}

		if try!(comma_delimiter(&mut p, &Token::Eof)) {
			break;
		}
	}

	Ok((nodes, group_name_to_node_names))
}

fn define_nodes(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree])
	-> Box<MacResult + 'static>
{
	let (nodes, mut group_name_to_node_names) = match parse_nodes(cx.new_parser_from_tts(args)) {
		Ok(res) => res,
		Err(mut e) => {
			e.emit();
			return DummyResult::any(sp);
		},
	};

	let builder = aster::AstBuilder::new().span(sp);

	let mut items = Vec::with_capacity(nodes.len() + group_name_to_node_names.len());

	let items_nodes = nodes.iter().map(|node| {
		let field_iter = node.fields.iter().map(|field| {
			let mut f = builder.struct_field(field.name);
			if field.is_public {
				f = f.pub_();
			}
			let ty = f.ty();

			match field.child_type {
				NodeChildType::Single => ty.box_().id(field.group),
				NodeChildType::Optional => ty.option().box_().id(field.group),
				NodeChildType::Multi =>
					ty
						.path()
							.global()
							.id("vtree")
							.id("key")
							.segment("KeyedNodes")
								.ty().id(field.group)
							.build()
						.build()
			}
		});


		let params_field = if let Some(ref ty) = node.params_type {
			vec![
				builder.struct_field("params").ty().build(ty.clone()),
			]
		} else {
			vec![]
		};

		builder
			.item()
				.pub_()
				.attr()
					.list("derive")
						.word("Debug")
						.word("Clone")
					.build()
				.struct_(node.name)
				.with_fields(params_field)
				.with_fields(field_iter)
			.build()
	});
	items.extend(items_nodes);

	let items_groups = group_name_to_node_names.iter().map(|(group, nodes)| {
		let var_iter = nodes.iter().map(|node| {
			builder
				.variant(node)
					.tuple()
						.ty()
						.id(node)
					.build()
		});

		builder
			.item()
				.pub_()
				.attr()
					.list("derive")
						.word("Debug")
						.word("Clone")
					.build()
				.enum_(group)
				.variant("Widget")
					.tuple()
						.ty()
						.box_()
						.path()
							.global()
							.id("vtree")
							.id("widget")
							.segment("WidgetDataTrait")
								.ty().id(group)
							.build()
						.build()
					.build()
				.with_variants(var_iter)
			.build()
	});
	items.extend(items_groups);

	let items_group_trait_impl = group_name_to_node_names.iter().map(|(group, _)| {
		builder
			.item()
				.impl_()
				.trait_()
					.global()
					.id("vtree")
					.id("group")
					.segment("Group")
					.build()
				.build()
				.ty().id(group)
		});
	items.extend(items_group_trait_impl);

	MacEager::items(SmallVector::many(items))
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
	reg.register_macro("define_nodes", define_nodes);
}
