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
        let fields_equal = node.fields.iter().map(|field| {
            let name_field_str = &field.name;
            let name_field = to_ident(&field.name);

            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        AllNodes::diff(
                            Some(&curr_node.#name_field),
                            Some(&last_node.#name_field),
                            &curr_path,
                            0,
                            ctx,
                            differ,
                        );
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        AllNodes::diff(
                            curr_node.#name_field.as_ref(),
                            last_node.#name_field.as_ref(),
                            &curr_path,
                            0,
                            ctx,
                            differ,
                        );
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        for diff in curr_node.#name_field.diff(&last_node.#name_field) {
                            match diff {
                                MultiDiff::Node(key, index, curr_child, last_child) =>
                                    AllNodes::diff(
                                        curr_child,
                                        last_child,
                                        &curr_path.add_key(key.clone()),
                                        index,
                                        ctx,
                                        differ,
                                    ),
                                MultiDiff::Reordered(indices) =>
                                    differ.diff(
                                        &curr_path,
                                        Diff::Reordered {
                                            indices: indices,
                                        }
                                    ),
                            }
                        }
                    }
                }
            }
        });

        let fields_added_vec: Vec<_> = node.fields
            .iter()
            .map(|field| {
                let name_field_str = &field.name;
                let name_field = to_ident(&field.name);

                match field.child_type {
                    NodeChildType::Single | NodeChildType::Optional => {
                        quote!{
                            let curr_path = path.add_field(#name_field_str);
                            AllNodes::diff(
                                Some(&curr_node.#name_field),
                                None,
                                &curr_path,
                                0,
                                ctx,
                                differ,
                            );
                        }
                    }
                    NodeChildType::Multi => {
                        quote!{
                            let curr_path = path.add_field(#name_field_str);
                            for (key, node) in curr_node.#name_field.iter() {
                                AllNodes::diff(
                                    Some(node),
                                    None,
                                    &curr_path.add_key(key.clone()),
                                    0,
                                    ctx,
                                    differ,
                                );
                            }
                        }
                    }
                }
            })
            .collect();
        let fields_added = &fields_added_vec[..];

        let fields_removed = node.fields.iter().map(|field| {
            let name_field_str = &field.name;
            let name_field = to_ident(&field.name);

            match field.child_type {
                NodeChildType::Single | NodeChildType::Optional => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        AllNodes::diff(
                            None,
                            Some(&last_node.#name_field),
                            &curr_path,
                            0,
                            ctx,
                            differ,
                        );
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        for (key, node) in last_node.#name_field.iter() {
                            AllNodes::diff(
                                None,
                                Some(node),
                                &curr_path.add_key(key.clone()),
                                0,
                                ctx,
                                differ,
                            );
                        }
                    }
                }
            }
        });

        let node_name = to_ident(&node.name);

        quote!{
            // equal types
            (
                Some(curr @ &AllNodes::#node_name(..)),
                Some(last @ &AllNodes::#node_name(..))
            ) => {
                // https://github.com/rust-lang/rust/pull/16053
                let (curr_node, last_node) = match (curr, last) {
                    (
                        &AllNodes::#node_name(ref curr_node),
                        &AllNodes::#node_name(ref last_node)
                    ) => (curr_node, last_node),
                    _ => unreachable!(),
                };

                // FIX: params are optional
                if curr_node.params != last_node.params {
                    differ.diff(
                        path,
                        Diff::ParamsChanged {
                            curr: curr,
                            last: last,
                        }
                    );
                }
                #(#fields_equal)*
            }

            // replaced
            (
                Some(curr @ &AllNodes::#node_name(..)),
                Some(last)
            ) => {
                let curr_node = match curr {
                    &AllNodes::#node_name(ref curr_node) => curr_node,
                    _ => unreachable!(),
                };

                AllNodes::diff(
                    None,
                    Some(last),
                    path,
                    index,
                    ctx,
                    differ,
                );

                differ.diff(
                    path,
                    Diff::Replaced {
                        index: index,
                        curr: curr,
                        last: last,
                    }
                );

                #(#fields_added)*
            }

            // added
            (
                Some(curr @ &AllNodes::#node_name(..)),
                None
            ) => {
                let curr_node = match curr {
                    &AllNodes::#node_name(ref curr_node) => curr_node,
                    _ => unreachable!(),
                };

                differ.diff(
                    path,
                    Diff::Added {
                        index: index,
                        curr: curr,
                    }
                );
                #(#fields_added)*
            }

            // removed
            (
                None,
                Some(last @ &AllNodes::#node_name(..))
            ) => {
                let last_node = match last {
                    &AllNodes::#node_name(ref last_node) => last_node,
                    _ => unreachable!(),
                };

                #(#fields_removed)*
                differ.diff(
                    path,
                    Diff::Removed {
                        index: index,
                        last: last,
                    }
                );
            }
        }
    });

    quote!{
        pub fn diff<'a, D>(
            curr: ::std::option::Option<&'a AllNodes>,
            last: ::std::option::Option<&'a AllNodes>,
            path: &::vtree::diff::Path,
            index: usize,
            ctx: &::vtree::diff::Context<AllNodes>,
            differ: &'a D,
        )
            where D: ::vtree::diff::Differ<'a, AllNodes>
        {
            use ::vtree::diff::Diff;
            use ::vtree::child::MultiDiff;

            match (curr, last) {
                #(#variants)*
                (Some(&AllNodes::Widget(_)), _) => panic!("curr can't be a AllNodes::Widget in diff"),
                (_, Some(&AllNodes::Widget(_))) => panic!("last can't be a AllNodes::Widget in diff"),
                (None, None) => panic!("curr and last can't be both Option::None"),
            }
        }
    }
}

fn gen_all_nodes_impl_visit_variants<'a>(pd: &'a ParsedData, is_enter: bool) -> impl Iterator<Item = Tokens> + 'a  {
    let name_visit = Ident::from(if is_enter {"visit_enter"} else {"visit_exit"});
    pd.nodes.iter().filter_map(move |node| {
        if node.fields.is_empty() {
            return None;
        }

        let fields = node.fields.iter().map(|field| {
            let name_field_str = &field.name;
            let name_field = to_ident(&field.name);

            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        if let Some(field) = &curr_node.#name_field {
                            field.#name_visit(&curr_path, 0, f);
                        }
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        &curr_node.#name_field.#name_visit(&curr_path, 0, f);
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        let it = curr_node.#name_field.iter_ordered().enumerate();
                        for (index, (key, node)) in it {
                            node.#name_visit(&curr_path.add_key(key.clone()), index, f);
                        }
                    }
                }
            }
        });

        let node_name = to_ident(&node.name);
        Some(quote!{
            &AllNodes::#node_name(ref curr_node) => {
                #(#fields)*
            }
        })
    })
}

fn gen_all_nodes_impl_visit(pd: &ParsedData) -> Tokens {
    let variants_enter = gen_all_nodes_impl_visit_variants(pd, true);
    let variants_exit = gen_all_nodes_impl_visit_variants(pd, false);

    quote!{
        pub fn visit_enter<F>(&self, path: &::vtree::diff::Path, index: usize, f: &mut F)
            where F: ::std::ops::FnMut(&::vtree::diff::Path, usize, &AllNodes)
        {
            f(path, index, self);
            match self {
                #(#variants_enter)*
                _ => (),
            }
        }

        pub fn visit_exit<F>(&self, path: &::vtree::diff::Path, index: usize, f: &mut F)
            where F: ::std::ops::FnMut(&::vtree::diff::Path, usize, &AllNodes)
        {
            match self {
                #(#variants_exit)*
                _ => (),
            }
            f(path, index, self);
        }
    }
}

fn gen_all_nodes_impl(pd: &ParsedData) -> Tokens {
    let expand_widgets = gen_all_nodes_impl_expand_widgets(pd);
    let diff = gen_all_nodes_impl_diff(pd);
    let visit = gen_all_nodes_impl_visit(pd);
    quote!{
        impl AllNodes {
            #expand_widgets
            #diff
            #visit
        }
    }
}

fn gen_node_constructor_fns<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
    pd.nodes.iter().map(|node| {
        let node_name = to_ident(&node.name);
        let node_name_sc = Ident::from(to_snake_case(&node.name));

        let maybe_params_generic = node.params_type.as_ref().map(|params_type| {
            let params_type_name = to_ident(params_type);
            quote!{
                P: ::std::convert::Into<#params_type_name>,
            }
        });
        let maybe_params_arg = node.params_type.as_ref().map(|_| {
            quote!{
                params: P,
            }
        });


        let field_arg_generics = node.fields.iter().enumerate().map(|(index, field)| {
            let group_name = to_ident(&field.group);
            let generic_name = Ident::from(format!("C{}", index));
            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        #generic_name: ::std::convert::Into<
                            ::vtree::child::Single<#group_name, AllNodes>
                        >,
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        #generic_name: ::std::convert::Into<
                            ::vtree::child::Optional<#group_name, AllNodes>
                        >,
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        #generic_name: ::std::convert::Into<
                            ::vtree::child::Multi<#group_name, AllNodes>
                        >,
                    }
                }
            }
        });
        let field_args = node.fields.iter().enumerate().map(|(index, field)| {
            let field_name_local = Ident::from(format!("child_{}", field.name));
            let generic_name = Ident::from(format!("C{}", index));
            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        #field_name_local: #generic_name,
                    }
                }
                NodeChildType::Optional => {
                    quote!{
                        #field_name_local: #generic_name,
                    }
                }
                NodeChildType::Multi => {
                    quote!{
                        #field_name_local: #generic_name,
                    }
                }
            }
        });

        let maybe_params_constr = node.params_type.as_ref().map(|_params_type| {
            quote!{
                params: params.into(),
            }
        });
        let field_constrs = node.fields.iter().map(|field| {
            let field_name_local = Ident::from(format!("child_{}", field.name));
            let field_name = to_ident(&field.name);
            quote!{
                #field_name: #field_name_local.into(),
            }
        });

        quote!{
            pub fn #node_name_sc<
                #maybe_params_generic
                #(#field_arg_generics)*
                R: ::std::convert::From<#node_name>,
            >(#maybe_params_arg #(#field_args)*) -> R {
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
        .chain(gen_group_from_node_impls("AllNodes", &pd.nodes[..]));
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
