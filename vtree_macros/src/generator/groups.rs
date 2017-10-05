use syn::Ident;
use quote::Tokens;
use parser::{ParsedData, ChildType, Node};
use std::iter::once;

pub fn gen_group_def<'a, IT>(group: &'a Ident, nodes: IT) -> Tokens
    where IT: Iterator<Item = &'a Node>
{
    let vars = nodes.map(|node| {
        match node {
            &Node::Normal(ref node) => {
                let node = &node.name;
                quote!{
                    #node(super::#node),
                }
            }
            &Node::Text => {
                quote!{
                    Text(::std::borrow::Cow<'static, str>),
                }
            }
        }

    });

    quote!{
        #[derive(Debug, Clone)]
        pub enum #group {
            #(#vars)*
            Widget(::std::boxed::Box<::vtree::widget::WidgetDataTrait<#group>>),
        }
    }
}

fn gen_all_nodes_impl_expand_widgets(pd: &ParsedData) -> Tokens {
    let variants = pd.nodes().map(|node| {
        let node = match node {
            &Node::Normal(ref node) => node,
            &Node::Text => {
                return quote!{
                    (&mut AllNodes::Text(..), _) => {}
                };
            }
        };

        let node_name = &node.name;

        let ty = if let Some((ty, _)) = node.child {
            ty
        } else {
            return quote!{
                (&mut AllNodes::#node_name(..), _) => {}
            };
        };

        let child_last_some = match ty {
            ChildType::Single => {
                quote!{
                    AllNodes::expand_widgets(
                        &mut curr_node.children,
                        Some(&last_node.children),
                        &path.add_empty(),
                    );
                }
            }
            ChildType::Optional => {
                quote!{
                    if let Some(children) = curr_node.children {
                        AllNodes::expand_widgets(
                            children,
                            last_node.children,
                            &path.add_empty(),
                        );
                    }
                }
            }
            ChildType::Multi => {
                quote!{
                    for (key, node) in curr_node.children.iter_mut() {
                        AllNodes::expand_widgets(
                            node,
                            last_node.children.get_by_key(key),
                            &path.add_key(key.clone()),
                        );
                    }
                }
            }
        };

        let child_last_none = match ty {
            ChildType::Single => {
                quote!{
                    AllNodes::expand_widgets(
                        &mut curr_node.children,
                        None,
                        &path.add_empty(),
                    );
                }
            }
            ChildType::Optional => {
                quote!{
                    if let Some(ref mut children) = curr_node.children {
                        AllNodes::expand_widgets(
                            children,
                            None,
                            &path.add_empty(),
                        );
                    }
                }
            }
            ChildType::Multi => {
                quote!{
                    for (key, node) in curr_node.children.iter_mut() {
                        AllNodes::expand_widgets(
                            node,
                            None,
                            &path.add_key(key.clone()),
                        );
                    }
                }
            }
        };

        quote!{
            (
                &mut AllNodes::#node_name(ref mut curr_node),
                Some(&AllNodes::#node_name(ref last_node))
            ) => {
                #child_last_some
            }
            (
                &mut AllNodes::#node_name(ref mut curr_node),
                _
            ) => {
                #child_last_none
            }
        }
    });

    quote!{
        pub fn expand_widgets(
            curr: &mut AllNodes,
            last: ::std::option::Option<&AllNodes>,
            path: &::vtree::diff::SimplePathFrame,
        ) {
            if let &mut AllNodes::Widget(..) = curr {
                let null_widget =
                    AllNodes::Widget(::std::boxed::Box::new(::vtree::widget::NullWidgetData));
                let widget_data = match ::std::mem::replace(curr, null_widget) {
                    AllNodes::Widget(widget_data) => widget_data,
                    _ => unreachable!(),
                };
                match widget_data.render() {
                    Some(result) => {
                        ::std::mem::replace(curr, result);
                    }
                    None => {
                        ::std::mem::replace(curr, last.unwrap().clone());
                        return;
                    }
                }
            }

            match (curr, last) {
                #(#variants)*
                (&mut AllNodes::Widget(_), _) => unreachable!(),
            }
        }
    }
}

fn gen_all_nodes_impl_diff(pd: &ParsedData) -> Tokens {
    let variants = pd.nodes().map(|node| {
        let node = match node {
            &Node::Normal(ref node) => node,
            &Node::Text => {
                return quote!{
                    (&AllNodes::Text(ref str_a), &AllNodes::Text(ref str_b)) => {
                        if str_a != str_b {
                            differ.diff_replaced(ctx, curr, last);
                        }
                    }
                    (&AllNodes::Text(..), _) => {
                        differ.diff_replaced(ctx, curr, last);
                    }
                };
            }
        };

        let maybe_child = node.child.as_ref().map(|&(ty, _)| {
            match ty {
                ChildType::Single => {
                    quote!{
                        AllNodes::diff(
                            &curr.add_empty(&curr_node.children),
                            &last.add_empty(&last_node.children),
                            ctx,
                            differ,
                        );
                    }
                }
                ChildType::Optional => {
                    quote!{
                        match (&curr_node.children, &last_node.children) {
                            (&Some(ref curr_child), &Some(ref last_child)) =>
                                AllNodes::diff(
                                    &curr.add_empty(curr_child),
                                    &last.add_empty(last_child),
                                    ctx,
                                    differ,
                                ),
                            (&Some(ref curr_child), None) => {
                                let c = curr.add_empty(curr_child);
                                differ.on_enter_curr(ctx, &c);
                                differ.diff_added(ctx, &c);
                                differ.on_exit_curr(ctx, &c);
                            }
                            (None, &Some(ref last_child)) => {
                                let l = last.add_empty(last_child);
                                differ.on_enter_last(ctx, &l);
                                differ.diff_removed(ctx, &l);
                                differ.on_exit_last(ctx, &l);
                            }
                            (None, None) => {}
                        }
                    }
                }
                ChildType::Multi => {
                    quote!{
                        let field_diff = curr_node.children.diff(&last_node.children);
                        for (key, curr_child, last_child) in field_diff {
                            match (curr_child, last_child) {
                                (Some((curr_index, curr_child)), Some((last_index, last_child))) =>
                                    AllNodes::diff(
                                        &curr.add_key(key.clone(), curr_index, curr_child),
                                        &last.add_key(key.clone(), last_index, last_child),
                                        ctx,
                                        differ
                                    ),
                                (Some((curr_index, curr_child)), None) => {}
                                    let c = curr.add_key(key.clone(), curr_index, curr_child);
                                    differ.on_enter_curr(ctx, &c);
                                    differ.diff_added(ctx, &c);
                                    differ.on_exit_curr(ctx, &c);
                                }
                                (None, Some((last_index, last_child))) =>
                                    let l = last.add_key(key.clone(), last_index, last_child);
                                    differ.on_enter_last(ctx, &l);
                                    differ.diff_removed(ctx, &l),
                                    differ.on_exit_last(ctx, &l);
                                (None, None) => unreachable!(),
                            }
                        }

                        let reordered = curr_node.children.diff_reordered(&last_node.children);
                        differ.diff_reordered(ctx, curr, reordered);
                    }
                }
            }
        });

        let node_name = &node.name;

        let maybe_params_cmp = node.params_ty.as_ref().map(|_| quote!{
            if curr_node.params != last_node.params {
                differ.diff_params_changed(ctx, curr, last);
            }
        });

        quote!{
            // equal types
            (
                &AllNodes::#node_name(ref curr_node),
                &AllNodes::#node_name(ref last_node)
            ) => {
                #maybe_params_cmp
                #maybe_child
            }

            // replaced
            (
                &AllNodes::#node_name(..),
                _
            ) => {
                differ.diff_replaced(ctx, curr, last);
            }
        }
    });

    quote!{
        pub fn diff<CTX, D>(
            curr: &::vtree::diff::PathFrame<AllNodes>,
            last: &::vtree::diff::PathFrame<AllNodes>,
            ctx: &mut ::vtree::diff::Context<CTX, AllNodes>,
            differ: &mut D,
        )
            where D: ::vtree::diff::Differ<CTX, AllNodes>
        {
            diff.on_enter_curr(ctx, curr);
            diff.on_enter_last(ctx, last);
            match (curr.node(), last.node()) {
                (&AllNodes::Widget(_), _) => panic!("curr isn't allowed to be a AllNodes::Widget in diff"),
                (_, &AllNodes::Widget(_)) => panic!("last isn't allowed to be a AllNodes::Widget in diff"),
                #(#variants)*
            }
            diff.on_exit_curr(ctx, curr);
            diff.on_exit_last(ctx, last);
        }
    }
}

fn gen_all_nodes_impl_visit_variants<'a>(pd: &'a ParsedData, is_enter: bool) -> impl Iterator<Item = Tokens> + 'a  {
    let name_visit = Ident::from(if is_enter {"visit_enter"} else {"visit_exit"});
    pd.normal_nodes().filter_map(move |node| {
        let ty = if let Some((ty, _)) = node.child {
            ty
        } else {
            return None;
        };
        let child = match ty {
            ChildType::Single => {
                quote!{
                    AllNodes::#name_visit(&curr.add_empty(&curr_node.children), f);
                }
            }
            ChildType::Optional => {
                quote!{
                    if let Some(ref node) = &curr_node.children {
                        AllNodes::#name_visit(&curr.add_empty(node), f);
                    }
                }
            }
            ChildType::Multi => {
                quote!{
                    let it = curr_node.children.iter().enumerate();
                    for (index, (key, node)) in it {
                        AllNodes::#name_visit(&curr.add_key(key.clone(), index, node), f);
                    }
                }
            }
        };

        let node_name = &node.name;
        Some(quote!{
            &AllNodes::#node_name(ref curr_node) => {
                #child
            }
        })
    })
}

fn gen_all_nodes_impl_visit(pd: &ParsedData) -> Tokens {
    let variants_enter = gen_all_nodes_impl_visit_variants(pd, true);
    let variants_exit = gen_all_nodes_impl_visit_variants(pd, false);

    // let variants = pd.normal_nodes().filter_map(move |node| {
    //     let ty = if let Some((ty, _)) = node.child {
    //         ty
    //     } else {
    //         return None;
    //     };
    //     let child = match ty {
    //         ChildType::Single => {
    //             quote!{
    //                 curr_node.children.visit(&path.add_empty(), 0, s, f);
    //             }
    //         }
    //         ChildType::Optional => {
    //             quote!{
    //                 if let Some(children) = &curr_node.children {
    //                     children.visit(&path.add_empty(), 0, s, f);
    //                 }
    //             }
    //         }
    //         ChildType::Multi => {
    //             quote!{
    //                 let it = curr_node.children.iter().enumerate();
    //                 for (index, (key, node)) in it {
    //                     node.visit(&path.add_key(key.clone()), index, s, f);
    //                 }
    //             }
    //         }
    //     };
    //
    //     let node_name = &node.name;
    //     Some(quote!{
    //         &AllNodes::#node_name(ref curr_node) => {
    //             #child
    //         }
    //     })
    // });

    quote!{
        // pub fn visit<S, F>(&self, path: &::vtree::diff::Path, index: usize, s: &mut S, f: &F)
        //     where
        //         F: ::std::ops::Fn(
        //             &::vtree::diff::Path,
        //             usize,
        //             &AllNodes,
        //             &mut S,
        //             &::std::ops::Fn(&mut S)
        //         ),
        // {
        //     let cb = &|s: &mut S| {
        //         match self {
        //             #(#variants)*
        //             _ => (),
        //         }
        //     };
        //     f(path, index, self, s, cb);
        // }

        pub fn visit_enter<F>(curr: &::vtree::diff::PathFrame<AllNodes>, f: &mut F)
            where F: ::std::ops::FnMut(&::vtree::diff::PathFrame<AllNodes>)
        {
            f(curr);
            match curr.node() {
                #(#variants_enter)*
                _ => (),
            }
        }

        pub fn visit_exit<F>(curr: &::vtree::diff::PathFrame<AllNodes>, f: &mut F)
            where F: ::std::ops::FnMut(&::vtree::diff::PathFrame<AllNodes>)
        {
            match curr.node() {
                #(#variants_exit)*
                _ => (),
            }
            f(curr);
        }
    }
}

pub fn gen_all_nodes_impl(pd: &ParsedData) -> Tokens {
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

pub fn gen_group_from_node_impls<'a, IT>(group: &'a Ident,
                                    nodes: IT)
                                    -> impl Iterator<Item = Tokens> + 'a
    where IT: Iterator<Item = &'a Node> + 'a
{
    once(quote!{
        impl <WD> ::std::convert::From<WD> for #group
            where WD: ::vtree::widget::WidgetDataTrait<#group> + 'static
        {
            fn from(widget_data: WD) -> #group {
                #group::Widget(::std::boxed::Box::new(widget_data))
            }
        }
    })
        .chain(nodes.map(move |node| {
            match node {
                &Node::Normal(ref node) => {
                    let node_name = &node.name;
                    quote!{
                        impl ::std::convert::From<super::#node_name> for #group {
                            fn from(node: super::#node_name) -> #group {
                                #group::#node_name(node)
                            }
                        }
                    }
                }
                &Node::Text => {
                    quote!{
                        impl ::std::convert::From<::std::borrow::Cow<'static, str>> for #group {
                            fn from(s: ::std::borrow::Cow<'static, str>) -> #group {
                                #group::Text(s)
                            }
                        }

                        impl ::std::convert::From<&'static str> for #group {
                            fn from(s: &'static str) -> #group {
                                #group::Text(s.into())
                            }
                        }

                        impl ::std::convert::From<::std::string::String> for #group {
                            fn from(s: ::std::string::String) -> #group {
                                #group::Text(s.into())
                            }
                        }
                    }
                }
            }

        }))
}

pub fn gen_all_nodes_from_group_impls<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
    pd.groups_nodes().map(|(group_name, nodes)| {
        let variants = nodes.map(|node| {
            match node {
                &Node::Normal(ref node) => {
                    let node_name = &node.name;
                    quote!{
                        #group_name::#node_name(node) => AllNodes::#node_name(node),
                    }
                }
                &Node::Text => {
                    quote!{
                        #group_name::Text(text) => AllNodes::Text(text),
                    }
                }
            }

        });

        quote!{
            impl ::std::convert::From<#group_name> for AllNodes {
                fn from(group: #group_name) -> AllNodes {
                    match group {
                        #(#variants)*
                        #group_name::Widget(_) => unimplemented!(),
                    }
                }
            }
        }
    })
}
