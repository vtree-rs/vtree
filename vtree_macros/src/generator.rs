use syntax::parse::{ParseSess, filemap_to_tts};
use syntax::tokenstream::TokenStream;
use regex::{Regex, Captures};
use syn::Ident;
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

fn gen_node_defs<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
    pd.nodes.iter().map(|node| {
        let fields = node.fields.iter().map(|field| {
            let name = &field.name;
            let group = &field.group;
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
            quote!{
                pub params: #params
            }
        });

        let name = &node.name;
        quote!{
            #[derive(Debug, Clone)]
            pub struct #name {
                #params_field,
                #(#fields)*
            }
        }
    })
}

fn gen_group_def(group: &Ident, nodes: &[Node]) -> Tokens {
    let vars = nodes.iter().map(|node| {
        let node = &node.name;
        quote!{
            #node(#node)
        }
    });

    quote!{
        #[derive(Debug, Clone)]
        pub enum #group {
            #(#vars,)*
            Widget(::std::boxed::Box<::vtree::widget::WidgetDataTrait<#group>>),
        }
    }
}

fn gen_all_nodes_impl_expand_widgets(pd: &ParsedData) -> Tokens {
    let variants = pd.nodes.iter().map(|node| {
        let node_name = &node.name;
        if node.fields.is_empty() {
            return quote!{
                AllNodes::#node_name(curr_node) => AllNodes::#node_name(curr_node),
            };
        }

        let fields_then = node.fields.iter().map(|field| {
            let name_field_str = field.name.as_ref();
            let name_field = &field.name;
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
            let name_field_str = field.name.as_ref();
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
            let field_name = &field.name;
            let field_name_local = Ident::from(format!("child_{}", field.name));
            quote!{
                #field_name: mut #field_name_local
            }
        });
        let construct_fields = node.fields.iter().map(|field| {
            let field_name = &field.name;
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
        let fields = node.fields.iter().map(|field| {
            let name_field_str = field.name.as_ref();
            let name_field = &field.name;

            match field.child_type {
                NodeChildType::Single => {
                    quote!{
                        let curr_path = path.add_field(#name_field_str);
                        AllNodes::diff(
                            &curr_node.#name_field,
                            &last_node.#name_field,
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
                        match (&curr_node.#name_field, &last_node.#name_field) {
                            (&Some(ref curr_child), &Some(ref last_child)) =>
                                AllNodes::diff(
                                    curr_child,
                                    last_child,
                                    &curr_path,
                                    0,
                                    ctx,
                                    differ,
                                ),
                            (&Some(ref curr_child), None) =>
                                differ.diff(
                                    &curr_path,
                                    Diff::Added {
                                        index: 0,
                                        curr: curr_child,
                                    }
                                ),
                            (None, &Some(ref last_child)) =>
                                differ.diff(
                                    &curr_path,
                                    Diff::Removed {
                                        index: 0,
                                        last: last_child,
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
                                MultiDiff::Node(key, index, curr_child, last_child) => {
                                    match (curr_child, last_child) {
                                        (Some(curr_child), Some(last_child)) =>
                                            AllNodes::diff(
                                                curr_child,
                                                last_child,
                                                &curr_path.add_key(key.clone()),
                                                index,
                                                ctx,
                                                differ,
                                            ),
                                        (Some(curr_child), None) =>
                                            differ.diff(
                                                &curr_path.add_key(key.clone()),
                                                Diff::Added {
                                                    index: index,
                                                    curr: curr_child,
                                                }
                                            ),
                                        (None, Some(last_child)) =>
                                            differ.diff(
                                                &curr_path.add_key(key.clone()),
                                                Diff::Removed {
                                                    index: index,
                                                    last: last_child,
                                                }
                                            ),
                                        (None, None) => unreachable!(),
                                    }
                                }
                                MultiDiff::Reordered(indices) => {
                                    differ.diff(
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

        let node_name = &node.name;

        let maybe_params_cmp = node.params_type.as_ref().map(|_| quote!{
            if curr_node.params != last_node.params {
                differ.diff(
                    path,
                    Diff::ParamsChanged {
                        curr: curr,
                        last: last,
                    }
                );
            }
        });

        quote!{
            // equal types
            (
                &AllNodes::#node_name(ref curr_node),
                &AllNodes::#node_name(ref last_node)
            ) => {
                #maybe_params_cmp
                #(#fields)*
            }

            // replaced
            (
                &AllNodes::#node_name(..),
                _
            ) => {
                differ.diff(
                    path,
                    Diff::Replaced {
                        index: index,
                        curr: curr,
                        last: last,
                    }
                );
            }
        }
    });

    quote!{
        pub fn diff<'a, D>(
            curr: &'a AllNodes,
            last: &'a AllNodes,
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
                (&AllNodes::Widget(_), _) => panic!("curr isn't allowed to be a AllNodes::Widget in diff"),
                (_, &AllNodes::Widget(_)) => panic!("last isn't allowed to be a AllNodes::Widget in diff"),
                #(#variants)*
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
            let name_field_str = field.name.as_ref();
            let name_field = &field.name;

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

        let node_name = &node.name;
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
        let node_name = &node.name;
        let node_name_sc = Ident::from(to_snake_case(&node.name.as_ref()));

        let maybe_params_generic = node.params_type.as_ref().map(|params_type| {
            quote!{
                P: ::std::convert::Into<#params_type>,
            }
        });
        let maybe_params_arg = node.params_type.as_ref().map(|_| {
            quote!{
                params: P,
            }
        });


        let field_arg_generics = node.fields.iter().enumerate().map(|(index, field)| {
            let group_name = &field.group;
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
            let field_name = &field.name;
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

fn gen_group_from_node_impls<'a>(group: &'a Ident,
                                 nodes: &'a [Node])
                                 -> impl Iterator<Item = Tokens> + 'a {
    once(quote!{
        impl <WD> ::std::convert::From<WD> for #group
            where WD: ::vtree::widget::WidgetDataTrait<#group> + 'static
        {
            fn from(widget_data: WD) -> #group {
                #group::Widget(Box::new(widget_data))
            }
        }
    })
        .chain(nodes.iter()
            .map(move |node| {
                let node_name = &node.name;
                quote!{
                        impl ::std::convert::From<#node_name> for #group {
                            fn from(node: #node_name) -> #group {
                                #group::#node_name(node)
                            }
                        }
                    }
            })
        )
}

fn gen_all_nodes_from_group_impls<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
    pd.group_name_to_nodes.iter().map(|(group, nodes)| {
        let variants = nodes.iter().map(|node| {
            let node_name = &node.name;
            quote!{
                #group::#node_name(node) => AllNodes::#node_name(node),
            }
        });

        quote!{
            impl ::std::convert::From<#group> for AllNodes {
                fn from(group: #group) -> AllNodes {
                    match group {
                        #(#variants)*
                    }
                }
            }
        }
    })
}

pub fn generate_defs(pd: ParsedData) -> TokenStream {
    let all_nodes_ident = Ident::new("AllNodes");
    let node_defs = gen_node_defs(&pd);
    let group_defs = pd.group_name_to_nodes
        .iter()
        .map(|(g, ns)| gen_group_def(g, &ns[..]))
        .chain(once(gen_group_def(&all_nodes_ident, &pd.nodes[..])));
    let all_nodes_impl = gen_all_nodes_impl(&pd);
    let node_constructor_fns = gen_node_constructor_fns(&pd);
    let group_from_node_impls = pd.group_name_to_nodes
        .iter()
        .flat_map(|(g, ns)| gen_group_from_node_impls(g, &ns[..]))
        .chain(gen_group_from_node_impls(&all_nodes_ident, &pd.nodes[..]));
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
    let source_str = defs.as_str();
    let sess = ParseSess::new();
    let filemap =
        sess.codemap().new_filemap("<procmacro_lex>".to_string(), None, source_str.to_owned());
    filemap_to_tts(&sess, filemap).into_iter().collect()
}
