use syntax::ext::base::ExtCtxt;
use syntax::parse::token::{Token, DelimToken};
use syntax::symbol::keywords::Keyword;
use syntax::symbol::Symbol;
use syntax::ast::Ident;
use rustc_errors::DiagnosticBuilder;
use syntax::parse::parser::Parser;
use std::collections::HashMap;
use syntax::tokenstream::TokenStream;
use syntax::ext::quote::rt::ToTokens;
use NodeChildType;
use Node;
use NodeChild;
use ParsedData;

struct MyKeyword {
	#[allow(dead_code)]
	ident: Ident,
}

fn mk_keyword(s: &str) -> Keyword {
	let kw = MyKeyword {
		ident: Ident::with_empty_ctxt(Symbol::intern(s)),
	};
	unsafe {
		::std::mem::transmute(kw)
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


pub fn parse_nodes<'a>(ctx: &ExtCtxt, mut p: Parser<'a>)
	-> Result<ParsedData, DiagnosticBuilder<'a>>
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

	Ok(ParsedData {
		nodes: nodes,
		group_name_to_node_names: group_name_to_node_names,
	})
}
