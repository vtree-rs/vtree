use syntax::tokenstream::TokenStream;
use proc_macro_tokens::parse::lex;
use regex::{Regex, Captures};
use quote::Ident;
use quote::Tokens;
use NodeChildType;
use ParsedData;
use Node;

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
	let diff_groups = pd.group_name_to_node_names.keys().map(|group| {
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
	let reorders = pd.nodes.iter().flat_map(|node| {
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
	let params_changes = pd.nodes.iter()
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

	quote!{
		pub trait Differ {
			#(#diff_groups)*
			#(#reorders)*
			#(#params_changes)*
		}
	}
}

fn gen_group_impl_expand_widgets(pd: &ParsedData, group: &str, nodes: &[&Node]) -> Tokens {
	let group_name = to_ident(group);
	let variants = nodes.iter().map(|node| {
		let node_name = to_ident(&node.name);

		// TODO: handle single & optional fields
		let fields_then = node.fields.iter().map(|field| {
			let name_field_str = &field.name;
			let name_field = to_ident(&field.name);
			quote!{
				let path_field = path.add_node_field(#name_field_str);
				curr_node.#name_field.inplace_map(|key, node| {
					node.expand_widgets(last_node.#name_field.get_by_key(key), &path_field.add_key(key.clone()))
				});
			}
		});

		let fields_else = node.fields.iter().map(|field| {
			let name_field_str = &field.name;
			let name_field = to_ident(&field.name);
			quote!{
				let path_field = path.add_node_field(#name_field_str);
				curr_node.#name_field.inplace_map(|key, node| {
					node.expand_widgets(None, &path_field.add_key(key.clone()))
				});
			}
		});

		quote!{
			#group_name::#node_name(ref mut curr_node) => {
				if let Some(&#group_name::#node_name(ref last_node)) = last {
					#(#fields_then)*
				} else {
					#(#fields_else)*
				}
			},
		}
	});

	quote!{
		pub fn expand_widgets(self, last: Option<&#group_name>, path: &diff::Path) -> #group_name {
			let mut curr = if let #group_name::Widget(widget_data) = self {
				match widget_data.render() {
					Some(result) => result,
					None => {
						let last = last.unwrap();
						if let &#group_name::Widget(..) = last {
							panic!("Widgets not allowed in last in `{}`", path);
						}
						return last.clone();
					}
				}
			} else {
				self
			};

			match curr {
				#(#variants)*
				#group_name::Widget(_) => unreachable!(),
			}

			curr
		}
	}
}

fn gen_group_impl_diff(pd: &ParsedData, group: &str, nodes: &[&Node]) -> Tokens {
	quote!{
	}
}

fn gen_group_impls<'a>(pd: &'a ParsedData) -> impl Iterator<Item=Tokens> + 'a {
	pd.group_name_to_node_names.iter().map(move |(group, node_names)| {
		let nodes: Vec<_> = node_names.iter()
			.filter_map(|name| {
				pd.nodes.iter().find(|n| &n.name == name)
			})
			.collect();
		let expand_widgets = gen_group_impl_expand_widgets(&pd, group, &nodes[..]);
		let diff = gen_group_impl_diff(&pd, group, &nodes[..]);
		let name = to_ident(group);
		quote!{
			impl #name {
				#expand_widgets
				#diff
			}
		}
	})
}

pub fn generate_defs(pd: ParsedData) -> TokenStream {
	let node_defs = gen_node_defs(&pd);
	let group_defs = gen_group_defs(&pd);
	let differ_def = gen_differ_def(&pd);
	let group_impls = gen_group_impls(&pd);
	let defs = quote!{
		#(#node_defs)*
		#(#group_defs)*
		#(#group_impls)*
		#differ_def
	};
	println!("{}", defs);
	lex(defs.as_str())
}
