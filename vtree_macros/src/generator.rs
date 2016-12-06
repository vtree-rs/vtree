use syntax::tokenstream::TokenStream;
use proc_macro_tokens::parse::lex;
use regex::{Regex, Captures};
use quote::Ident;
use quote::Tokens;
use NodeChildType;
use ParsedData;

fn to_snake_case(s: &str) -> String {
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

fn to_ident(s: &str) -> Ident {
	Ident::from(s)
}

fn gen_node_defs<'a>(pd: &'a ParsedData) -> impl Iterator<Item=Tokens> + 'a {
	pd.nodes.iter().map(|node| {
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
	})
}

fn gen_group_defs<'a>(pd: &'a ParsedData) -> impl Iterator<Item=Tokens> + 'a {
	pd.group_name_to_node_names.iter().map(|(group, nodes)| {
		let vars = nodes.iter().map(|node| {
			let node = to_ident(node);
			quote!{
				#node(#node)
			}
		});

		let name = to_ident(group);
		quote!{
			#[derive(Debug, Clone)]
			pub enum #name {
				Widget(::std::boxed::Box<::vtree::widget::WidgetDataTrait<#name>>),
				#(#vars,)*
			}
		}
	})
}

fn gen_differ_def(pd: &ParsedData) -> Tokens {
	let differ_element_diff_groups = pd.group_name_to_node_names.keys().map(|group| {
		let name_diff_fn = Ident::from(format!("diff_{}", to_snake_case(group)));
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
	let differ_element_reorders = pd.nodes.iter().flat_map(|node| {
		let name_node_sc = to_snake_case(&node.name);
		node.fields.iter().map(move |field| {
			let name_fn = Ident::from(format!("reorder_{}_{}",
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
	let differ_element_params_changed_nodes = pd.nodes.iter()
		.filter(|node| node.params_type.is_some())
		.map(|node| {
			let name_node = to_ident(&node.name);
			let name_fn = Ident::from(format!("params_changed_{}", to_snake_case(&node.name)));
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

	quote!{
		pub trait Differ {
			#(#differ_elements)*
		}
	}
}

pub fn generate_defs(pd: ParsedData) -> TokenStream {
	let node_defs = gen_node_defs(&pd);
	let group_defs = gen_group_defs(&pd);
	let differ_def = gen_differ_def(&pd);

	let defs = quote!{
		#(#node_defs)*
		#(#group_defs)*
		#differ_def
	};
	println!("{}", defs);
	lex(defs.as_str())
}
