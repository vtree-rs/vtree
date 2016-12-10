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
        static ref RE: Regex = Regex::new("([a-z]|^)([A-Z])").unwrap();
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

fn gen_node_defs<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
    pd.nodes.iter().map(|node| {
        let fields = node.fields.iter().map(|field| {
            let name = to_ident(&field.name);
            let group = to_ident(&field.group);
            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        pub #name: ::std::boxed::Box<#group>
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        pub #name: ::std::option::Option<::std::boxed::Box<#group>>
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        pub #name: ::vtree::key::KeyedNodes<#group>
                    }
                }
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

fn gen_group_defs<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
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
            fn #name_diff_fn(
                &self,
                path: &::vtree::diff::Path,
                curr: &#name_group,
                diff: ::vtree::diff::Diff,
            );
        }
    });
    let reorders = pd.nodes.iter().flat_map(|node| {
        let name_node = to_ident(&node.name);
        let name_node_sc = to_snake_case(&node.name);
        node.fields.iter().map(move |field| {
            let name_fn =
                Ident::from(format!("reorder_{}_{}", name_node_sc, to_snake_case(&field.name)));
            quote!{
                fn #name_fn(
                    &self,
                    path: &::vtree::diff::Path,
                    parent: &#name_node,
                    index_curr: usize,
                    index_last: usize,
                );
            }
        })
    });
    let params_changes = pd.nodes
        .iter()
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

fn gen_group_impl_expand_widgets(group: &str, nodes: &[&Node]) -> Tokens {
    let group_name = to_ident(group);
    let variants = nodes.iter().map(|node| {
        let node_name = to_ident(&node.name);

        let fields_then = node.fields.iter().map(|field| {
            let name_field_str = &field.name;
            let name_field = to_ident(&field.name);
            let field_name_local = Ident::from(format!("child_{}", field.name));
            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        let path_field = path.add_node_field(#name_field_str);
                        #field_name_local = #field_name_local.expand_widgets(last_node.#name_field, &path_field);
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        let path_field = path.add_node_field(#name_field_str);
                        #field_name_local = if let Some(field) = #field_name_local {
                            field.expand_widgets(last_node.#name_field, &path_field);
                        };
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        let path_field = path.add_node_field(#name_field_str);
                        #field_name_local.inplace_map(|key, node| {
                            node.expand_widgets(last_node.#name_field.get_by_key(key), &path_field.add_key(key.clone()))
                        });
                    }
                }
            }
        });

        let fields_else = node.fields.iter().map(|field| {
            let name_field_str = &field.name;
            let field_name_local = Ident::from(format!("child_{}", field.name));
            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        let path_field = path.add_node_field(#name_field_str);
                        #field_name_local = #field_name_local.expand_widgets(None, &path_field);
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        let path_field = path.add_node_field(#name_field_str);
                        #field_name_local = if let Some(field) = #field_name_local {
                            field.expand_widgets(None, &path_field);
                        };
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        let path_field = path.add_node_field(#name_field_str);
                        #field_name_local.inplace_map(|key, node| {
                            node.expand_widgets(None, &path_field.add_key(key.clone()))
                        });
                    }
                }
            }
        });

        let destruct_fields = node.fields.iter().map(|field| {
            let field_name = to_ident(&field.name);
            let field_name_local = Ident::from(format!("child_{}", field.name));
            quote!{
                #field_name: mut #field_name_local
            }
        });
        let construct_fields = node.fields.iter().map(|field| {
            let field_name = to_ident(&field.name);
            let field_name_local = Ident::from(format!("child_{}", field.name));
            quote!{
                #field_name: #field_name_local
            }
        });
        let de_con_struct_params = if node.params_type.is_some() {
            Some(quote!{
                params: curr_params,
            })
        } else {
            None
        };

        quote!{
            #group_name::#node_name(#node_name{#(#destruct_fields,)* #de_con_struct_params}) => {
                if let Some(&#group_name::#node_name(ref last_node)) = last {
                    #(#fields_then)*
                } else {
                    #(#fields_else)*
                }
                #group_name::#node_name(#node_name{
                    #(#construct_fields,)*
                    #de_con_struct_params
                })
            },
        }
    });

    quote!{
        pub fn expand_widgets(self, last: Option<&#group_name>, path: &::vtree::diff::Path) -> #group_name {
            let curr = if let #group_name::Widget(widget_data) = self {
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
        }
    }
}

fn gen_group_impl_diff(group: &str, nodes: &[&Node]) -> Tokens {
    let group_name = to_ident(group);
    let diff_group = Ident::from(format!("diff_{}", to_snake_case(group)));

    let variants = nodes.iter().map(|node| {
        // TODO: handle single & optional fields
        let fields = node.fields.iter().map(|field| {
            let name_field_str = &field.name;
            let name_field = to_ident(&field.name);
            let reorder_children = Ident::from(format!("reorder_{}_{}",
                to_snake_case(&node.name),
                to_snake_case(&field.name),
            ));

            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        let curr_path = path.add_node_field(#name_field_str);
                        curr_node.#name_field.diff(&curr_path.add_key(key.clone()), last_node.#name_field, ctx);
                    }
                }
                NodeChildType::Optional => {
                    let diff_group_child = Ident::from(format!("diff_{}",
                                                               to_snake_case(&field.group)));
                    quote!{
                        let curr_path = path.add_node_field(#name_field_str);
                        if let Some(curr_child) = curr_node.#name_field {
                            if let Some(last_child) = last_node.#name_field {
                                curr_child.diff(&curr_path, last_child, ctx);
                            } else {
                                ctx.differ.#diff_group_child(&curr_path, &curr_child, Diff::Added);
                            }
                        } else {
                            if let Some(last_child) = last_node.#name_field {
                                ctx.differ.#diff_group_child(&curr_path, &last_child, Diff::Removed);
                            }
                        }
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        let curr_path = path.add_node_field(#name_field_str);
                        for diff in curr_node.#name_field.diff(&last_node.#name_field) {
                            match diff {
                                KeyedDiff::Added(key, _index, node) => {
                                    ctx.differ.#diff_group(&curr_path.add_key(key.clone()), &node, Diff::Added);
                                },
                                KeyedDiff::Removed(key, _index, node) => {
                                    ctx.differ.#diff_group(&curr_path.add_key(key.clone()), &node, Diff::Removed);
                                },
                                KeyedDiff::Unchanged(key, _index, curr_child, last_child) => {
                                    curr_child.diff(&curr_path.add_key(key.clone()), last_child, ctx);
                                },
                                KeyedDiff::Reordered(i_cur, i_last) => {
                                    ctx.differ.#reorder_children(path, &curr_node, i_cur, i_last);
                                },
                            }
                        }
                    }
                }
            }
        });

        let node_name = to_ident(&node.name);
        let params_changed = Ident::from(format!("params_changed_{}", to_snake_case(&node.name)));
        quote!{
            &#group_name::#node_name(ref curr_node) => {
                if let &#group_name::#node_name(ref last_node) = last {
                    if curr_node.params != last_node.params {
                        ctx.differ.#params_changed(path, curr_node, &last_node);
                    }
                    #(#fields)*
                } else {
                    // TODO: call node removed hook
                    ctx.differ.#diff_group(path, &self, Diff::Replaced);
                }
            },
        }
    });

    quote!{
        pub fn diff<D: Differ>(
            &self,
            path: &::vtree::diff::Path,
            last: &#group_name,
            ctx: &::vtree::diff::Context<D>,
        ) {
            use ::vtree::diff::Diff;
            use ::vtree::key::KeyedDiff;

            match self {
                #(#variants)*
                &#group_name::Widget(_) => unreachable!(),
            }
        }
    }
}

fn gen_group_impls<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
    pd.group_name_to_node_names.iter().map(move |(group, node_names)| {
        let nodes: Vec<_> = node_names.iter()
            .filter_map(|name| pd.nodes.iter().find(|n| &n.name == name))
            .collect();
        let expand_widgets = gen_group_impl_expand_widgets(group, &nodes[..]);
        let diff = gen_group_impl_diff(group, &nodes[..]);
        let name = to_ident(group);
        quote!{
            impl #name {
                #expand_widgets
                #diff
            }
        }
    })
}

fn gen_node_constructor_fns<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
    pd.nodes.iter().map(|node| {
        let node_name = to_ident(&node.name);
        let node_name_sc = Ident::from(to_snake_case(&node.name));

        let maybe_params_arg = node.params_type.as_ref().map(|params_type| {
            let params_type_name = to_ident(params_type);
            quote!{
                params: #params_type_name,
            }
        });
        let field_args = node.fields.iter().map(|field| {
            let field_name_local = Ident::from(format!("child_{}", field.name));
            let group_name = to_ident(&field.group);
            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        #field_name_local: #group_name,
                    }
                },
                NodeChildType::Optional => {
                    quote!{
                        #field_name_local: ::std::option::Option<#group_name>,
                    }
                },
                NodeChildType::Multi => {
                    quote!{
                        #field_name_local: ::vtree::key::KeyedNodes<#group_name>,
                    }
                },
            }

        });

        let maybe_params_constr = node.params_type.as_ref().map(|_params_type| {
            quote!{
                params: params,
            }
        });
        let field_constrs = node.fields.iter().map(|field| {
            let field_name_local = Ident::from(format!("child_{}", field.name));
            let field_name = to_ident(&field.name);
            quote!{
                #field_name: #field_name_local,
            }
        });

        quote!{
            pub fn #node_name_sc<T>(#maybe_params_arg #(#field_args)*) -> T
                where T: ::std::convert::From<#node_name>
            {
                ::std::convert::From::from(#node_name {
                    #maybe_params_constr
                    #(#field_constrs)*
                })
            }
        }
    })
}

fn gen_group_from_node_impls<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
    use std::iter::once;
    pd.group_name_to_node_names.iter().flat_map(|(group, nodes)| {
        let group_name = to_ident(group);
        once(quote!{
            impl <WD> ::std::convert::From<WD> for #group_name
                where WD: ::vtree::widget::WidgetDataTrait<#group_name> + 'static
            {
                fn from(widget_data: WD) -> #group_name {
                    #group_name::Widget(Box::new(widget_data))
                }
            }
        })
        .chain(nodes.iter().map(move |node| {
            let node_name = to_ident(node);
            quote!{
                impl ::std::convert::From<#node_name> for #group_name {
                    fn from(node: #node_name) -> #group_name {
                        #group_name::#node_name(node)
                    }
                }
            }
        }))
    })
}

pub fn generate_defs(pd: ParsedData) -> TokenStream {
    let node_defs = gen_node_defs(&pd);
    let group_defs = gen_group_defs(&pd);
    let differ_def = gen_differ_def(&pd);
    let group_impls = gen_group_impls(&pd);
    let node_constructor_fns = gen_node_constructor_fns(&pd);
    let group_from_node_impls = gen_group_from_node_impls(&pd);
    let defs = quote!{
        #(#node_defs)*
        #(#group_defs)*
        #(#group_impls)*
        #differ_def
        #(#node_constructor_fns)*
        #(#group_from_node_impls)*
    };
    println!("{}", defs);
    lex(defs.as_str())
}
