use syntax::tokenstream::TokenStream;
use proc_macro_tokens::parse::lex;
use regex::{Regex, Captures};
use quote::Ident;
use quote::Tokens;
use NodeChildType;
use ParsedData;
use Node;
use std::iter::once;

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
                        pub #name: ::vtree::child::Single<#group, AllNodes>,
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        pub #name: ::vtree::child::Option<#group, AllNodes>,
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        pub #name: ::vtree::child::Multi<#group, AllNodes>,
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
                #(#fields)*
            }
        }
    })
}

fn gen_group_def(group: &str, nodes: &[Node]) -> Tokens {
    let vars = nodes.iter().map(|node| {
        let node = to_ident(&node.name);
        quote!{
            #node(#node)
        }
    });

    let name = to_ident(group);
    quote!{
        #[derive(Debug, Clone)]
        pub enum #name {
            #(#vars,)*
            Widget(::std::boxed::Box<::vtree::widget::WidgetDataTrait<#name>>),
        }
    }
}

fn gen_all_nodes_impl_expand_widgets(pd: &ParsedData) -> Tokens {
    let variants = pd.nodes.iter().map(|node| {
        let node_name = to_ident(&node.name);

        if node.fields.is_empty() {
            return quote!{
                AllNodes::#node_name(curr_node) => AllNodes::#node_name(curr_node),
            };
        }

        let fields_then = node.fields.iter().map(|field| {
            let name_field_str = &field.name;
            let name_field = to_ident(&field.name);
            let field_name_local = Ident::from(format!("child_{}", field.name));
            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        let path_field = path.add_field(#name_field_str);
                        #field_name_local = #field_name_local.expand_widgets(last_node.#name_field, &path_field);
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        let path_field = path.add_field(#name_field_str);
                        #field_name_local = if let Some(field) = #field_name_local {
                            field.expand_widgets(last_node.#name_field, &path_field);
                        };
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        let path_field = path.add_field(#name_field_str);
                        #field_name_local.inplace_map(|key, node| {
                            node.expand_widgets(
                                last_node.#name_field.get_by_key(key),
                                &path_field.add_key(key.clone())
                            )
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
                        let path_field = path.add_field(#name_field_str);
                        #field_name_local = #field_name_local.expand_widgets(None, &path_field);
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        let path_field = path.add_field(#name_field_str);
                        #field_name_local = if let Some(field) = #field_name_local {
                            field.expand_widgets(None, &path_field);
                        };
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        let path_field = path.add_field(#name_field_str);
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
        let de_con_struct_params = node.params_type.as_ref().map(|_| {
            quote!{
                params: curr_params,
            }
        });

        quote!{
            AllNodes::#node_name(#node_name{#(#destruct_fields,)* #de_con_struct_params}) => {
                if let Some(&AllNodes::#node_name(ref last_node)) = last {
                    #(#fields_then)*
                } else {
                    #(#fields_else)*
                }
                AllNodes::#node_name(#node_name{
                    #(#construct_fields,)*
                    #de_con_struct_params
                })
            },
        }
    });

    quote!{
        pub fn expand_widgets(
            self,
            last: ::std::option::Option<&AllNodes>,
            path: &::vtree::diff::Path
        ) -> AllNodes {
            let curr = if let AllNodes::Widget(widget_data) = self {
                match widget_data.render() {
                    Some(result) => result,
                    None => {
                        let last = last.unwrap();
                        if let &AllNodes::Widget(..) = last {
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
                AllNodes::Widget(_) => unreachable!(),
            }
        }
    }
}

fn gen_all_nodes_impl_diff(pd: &ParsedData) -> Tokens {
    let variants = pd.nodes.iter().map(|node| {
        // TODO: handle single & optional fields
        let fields = node.fields.iter().map(|field| {
            let name_field_str = &field.name;
            let name_field = to_ident(&field.name);

            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        curr_node.#name_field.diff(&curr_path.add_key(key.clone()), last_node.#name_field, ctx);
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        match (*curr_node.#name_field, *last_node.#name_field) {
                            (Some(curr_child), Some(last_child)) =>
                                curr_child.diff(&curr_path, last_child, ctx),
                            (Some(curr_child), None) =>
                                ctx.differ.diff(
                                    &curr_path,
                                    Diff::Added {
                                        index: 0,
                                        curr: &curr_child,
                                    }
                                ),
                            (None, Some(last_child)) =>
                                ctx.differ.diff(
                                    &curr_path,
                                    Diff::Removed {
                                        index: 0,
                                        last: &curr_child,
                                    }
                                ),
                            (None, None) => {}
                        }
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        for diff in curr_node.#name_field.diff(&last_node.#name_field) {
                            match diff {
                                MultiDiff::Added(key, index, node) => {
                                    ctx.differ.diff(
                                        &curr_path.add_key(key.clone()),
                                        Diff::Added {
                                            index: index,
                                            curr: node,
                                        }
                                    );
                                }
                                MultiDiff::Removed(key, index, node) => {
                                    ctx.differ.diff(
                                        &curr_path.add_key(key.clone()),
                                        Diff::Removed {
                                            index: index,
                                            last: node,
                                        }
                                    );
                                }
                                MultiDiff::Unchanged(key, _index, curr_child, last_child) =>
                                    curr_child.diff(&curr_path.add_key(key.clone()), last_child, ctx),
                                MultiDiff::Reordered(indices) => {
                                    ctx.differ.diff(
                                        &curr_path,
                                        Diff::Reordered {
                                            indices: indices,
                                        }
                                    );
                                }
                            }
                        }
                    }
                }
            }
        });

        let node_name = to_ident(&node.name);
        quote!{
            &AllNodes::#node_name(ref curr_node) => {
                if let &AllNodes::#node_name(ref last_node) = last {
                    if curr_node.params != last_node.params {
                        ctx.differ.diff(path, Diff::ParamsChanged {
                            curr: self,
                            last: last,
                        });
                    }
                    #(#fields)*
                } else {
                    // TODO: call node removed hook
                    ctx.differ.diff(path, Diff::Replaced {
                        curr: self,
                        last: last,
                    });
                }
            },
        }
    });

    quote!{
        pub fn diff<'a, D: ::vtree::diff::Differ<'a, AllNodes>>(
            &'a self,
            path: &::vtree::diff::Path,
            last: &'a AllNodes,
            ctx: &'a ::vtree::diff::Context<'a, AllNodes, D>,
        ) {
            use ::vtree::diff::Diff;
            use ::vtree::child::MultiDiff;

            match self {
                #(#variants)*
                &AllNodes::Widget(_) => unreachable!(),
            }
        }
    }
}

fn gen_all_nodes_impl(pd: &ParsedData) -> Tokens {
    let expand_widgets = gen_all_nodes_impl_expand_widgets(pd);
    let diff = gen_all_nodes_impl_diff(pd);
    quote!{
        impl AllNodes {
            #expand_widgets
            #diff
        }
    }
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
                        #field_name_local: ::vtree::child::Single<#group_name, AllNodes>,
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        #field_name_local: ::vtree::child::Optional<#group_name, AllNodes>,
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        #field_name_local: ::vtree::child::Multi<#group_name, AllNodes>,
                    }
                }
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

fn gen_group_from_node_impls<'a>(group: &'a str,
                                 nodes: &'a [Node])
                                 -> impl Iterator<Item = Tokens> + 'a {
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
        .chain(nodes.iter()
            .map(move |node| {
                let node_name = to_ident(&node.name);
                quote!{
                        impl ::std::convert::From<#node_name> for #group_name {
                            fn from(node: #node_name) -> #group_name {
                                #group_name::#node_name(node)
                            }
                        }
                    }
            })
        )
}

fn gen_all_nodes_from_group_impls<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
    pd.group_name_to_nodes.iter().map(|(group, nodes)| {
        let group_name = to_ident(group);
        let variants = nodes.iter().map(|node| {
            let node_name = to_ident(&node.name);
            quote!{
                #group_name::#node_name(node) => AllNodes::#node_name(node),
            }
        });

        quote!{
            impl ::std::convert::From<#group_name> for AllNodes {
                fn from(group: #group_name) -> AllNodes {
                    match group {
                        #(#variants)*
                    }
                }
            }
        }
    })
}

pub fn generate_defs(pd: ParsedData) -> TokenStream {
    let node_defs = gen_node_defs(&pd);
    let group_defs = pd.group_name_to_nodes
        .iter()
        .map(|(g, ns)| gen_group_def(g, &ns[..]))
        .chain(once(gen_group_def("AllNodes", &pd.nodes[..])));
    let all_nodes_impl = gen_all_nodes_impl(&pd);
    let node_constructor_fns = gen_node_constructor_fns(&pd);
    let group_from_node_impls = pd.group_name_to_nodes
        .iter()
        .flat_map(|(g, ns)| gen_group_from_node_impls(g, &ns[..]))
        .chain(once(()).flat_map(|_| gen_group_from_node_impls("AllNodes", &pd.nodes[..])));
    let all_nodes_from_group_impls = gen_all_nodes_from_group_impls(&pd);
    let defs = quote!{
        #(#node_defs)*
        #(#group_defs)*
        #all_nodes_impl
        #(#node_constructor_fns)*
        #(#group_from_node_impls)*
        #(#all_nodes_from_group_impls)*
    };
    println!("{}", defs);
    lex(defs.as_str())
}
