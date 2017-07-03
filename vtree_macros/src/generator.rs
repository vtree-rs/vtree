use syn::Ident;
use quote::Tokens;
use parser::{ParsedData, ChildType, Node, NodeNormal, Child};
use std::iter::once;

fn gen_node_def_impl(node: &NodeNormal, pd: &ParsedData) -> Tokens {
    let name = &node.name;

    let maybe_params_arg = node.params_ty.as_ref().map(|ty| {
        let maybe_comma = node.child.as_ref().map(|_| quote!{,});
        quote!{
            params: #ty #maybe_comma
        }
    });

    let maybe_params_constr = node.params_ty.as_ref().map(|_| {
        quote!{
            params: params,
        }
    });

    let maybe_child_arg = node.child.as_ref().map(|&(ty, ref name)| {
        let name = match name {
            &Child::Node(ref name) => {
                match pd.node_by_name(name) {
                    Some(&Node::Normal(..)) => quote!{#name},
                    Some(&Node::Text) => quote!{::std::borrow::Cow<'static, str>},
                    None => unreachable!(),
                }
            }
            &Child::Group(ref name) => quote!{groups::#name},
        };
        let ty = match ty {
            ChildType::Single => {
                quote!{
                    ::vtree::child::Single<#name, groups::AllNodes>,
                }
            }
            ChildType::Optional => {
                quote!{
                    ::vtree::child::Option<#name, groups::AllNodes>,
                }
            }
            ChildType::Multi => {
                quote!{
                    ::vtree::child::Multi<#name, groups::AllNodes>,
                }
            }
        };
        quote!{
            children: #ty
        }
    });

    let maybe_child_constr = node.child.as_ref().map(|_| {
        quote!{
            children: children,
        }
    });

    quote!{
        impl #name {
            pub fn new(#maybe_params_arg #maybe_child_arg) -> #name {
                #name {
                    #maybe_params_constr
                    #maybe_child_constr
                }
            }

            pub fn builder() -> builders::#name {
                builders::#name::new()
            }
        }
    }
}

fn gen_node_defs<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
    pd.normal_nodes().map(move |node| {
        let maybe_child = node.child.as_ref().map(|&(ty, ref name)| {
            let name = match name {
                &Child::Node(ref name) => {
                    match pd.node_by_name(name) {
                        Some(&Node::Normal(..)) => quote!{#name},
                        Some(&Node::Text) => quote!{::std::borrow::Cow<'static, str>},
                        None => unreachable!(),
                    }
                }
                &Child::Group(ref name) => quote!{groups::#name},
            };
            match ty {
                ChildType::Single => {
                    quote!{
                        pub child: ::vtree::child::Single<#name, groups::AllNodes>,
                    }
                }
                ChildType::Optional => {
                    quote!{
                        pub child: ::vtree::child::Option<#name, groups::AllNodes>,
                    }
                }
                ChildType::Multi => {
                    quote!{
                        pub children: ::vtree::child::Multi<#name, groups::AllNodes>,
                    }
                }
            }
        });

        let maybe_params = node.params_ty.as_ref().map(|params| {
            quote!{
                pub params: #params,
            }
        });

        let name = &node.name;
        let node_impl = gen_node_def_impl(node, pd);
        quote!{
            #[derive(Debug, Clone)]
            pub struct #name {
                #maybe_child
                #maybe_params
            }

            #node_impl
        }
    })
}

fn gen_group_def<'a, IT>(group: &'a Ident, nodes: IT) -> Tokens
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
                        &path
                    );
                }
            }
            ChildType::Optional => {
                quote!{
                    if let Some(children) = curr_node.children {
                        AllNodes::expand_widgets(children, last_node.children, &path);
                    }
                }
            }
            ChildType::Multi => {
                quote!{
                    for (key, node) in curr_node.children.iter_mut() {
                        AllNodes::expand_widgets(
                            node,
                            last_node.children.get_by_key(key),
                            &path.add(key.clone())
                        );
                    }
                }
            }
        };

        let child_last_none = match ty {
            ChildType::Single => {
                quote!{
                    AllNodes::expand_widgets(&mut curr_node.children, None, &path);
                }
            }
            ChildType::Optional => {
                quote!{
                    if let Some(ref mut children) = curr_node.children {
                        AllNodes::expand_widgets(children, None, &path);
                    }
                }
            }
            ChildType::Multi => {
                quote!{
                    for (key, node) in curr_node.children.iter_mut() {
                        AllNodes::expand_widgets(node, None, &path.add(key.clone()));
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
            path: &::vtree::diff::Path
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
                            differ.diff_params_changed(path, curr, last);
                        }
                    }
                    (&AllNodes::Text(..), _) => {
                        differ.diff_replaced(path, index, curr, last);
                    }
                };
            }
        };

        let maybe_child = node.child.as_ref().map(|&(ty, _)| {
            match ty {
                ChildType::Single => {
                    quote!{
                        AllNodes::diff(
                            &curr_node.child,
                            &last_node.child,
                            &path,
                            0,
                            ctx,
                            differ,
                        );
                    }
                }
                ChildType::Optional => {
                    quote!{
                        match (&curr_node.child, &last_node.child) {
                            (&Some(ref curr_child), &Some(ref last_child)) =>
                                AllNodes::diff(
                                    curr_child,
                                    last_child,
                                    &path,
                                    0,
                                    ctx,
                                    differ,
                                ),
                            (&Some(ref curr_child), None) =>
                                differ.diff_added(&path, 0, curr_child),
                            (None, &Some(ref last_child)) =>
                                differ.diff_removed(&path, 0, last_child),
                            (None, None) => {}
                        }
                    }
                }
                ChildType::Multi => {
                    quote!{
                        let field_diff = curr_node.children.diff(&last_node.children);
                        for (key, index, curr_child, last_child) in field_diff {
                            match (curr_child, last_child) {
                                (Some(curr_child), Some(last_child)) =>
                                    AllNodes::diff(
                                        curr_child,
                                        last_child,
                                        &path.add(key.clone()),
                                        index,
                                        ctx,
                                        differ,
                                    ),
                                (Some(curr_child), None) =>
                                    differ.diff_added(&path.add(key.clone()), index, curr_child),
                                (None, Some(last_child)) =>
                                    differ.diff_removed(&path.add(key.clone()), index, last_child),
                                (None, None) => unreachable!(),
                            }
                        }

                        let reordered = curr_node.children.diff_reordered(&last_node.children);
                        differ.diff_reordered(&path, reordered);
                    }
                }
            }
        });

        let node_name = &node.name;

        let maybe_params_cmp = node.params_ty.as_ref().map(|_| quote!{
            if curr_node.params != last_node.params {
                differ.diff_params_changed(path, curr, last);
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
                differ.diff_replaced(path, index, curr, last);
            }
        }
    });

    quote!{
        pub fn diff<D>(
            curr: &AllNodes,
            last: &AllNodes,
            path: &::vtree::diff::Path,
            index: usize,
            ctx: &::vtree::diff::Context<AllNodes>,
            differ: &D,
        )
            where D: ::vtree::diff::Differ<AllNodes>
        {
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
    pd.normal_nodes().filter_map(move |node| {
        let ty = if let Some((ty, _)) = node.child {
            ty
        } else {
            return None;
        };
        let child = match ty {
            ChildType::Single => {
                quote!{
                    &curr_node.child.#name_visit(&path, 0, f);
                }
            }
            ChildType::Optional => {
                quote!{
                    if let Some(child) = &curr_node.child {
                        child.#name_visit(&path, 0, f);
                    }
                }
            }
            ChildType::Multi => {
                quote!{
                    let it = curr_node.children.iter().enumerate();
                    for (index, (key, node)) in it {
                        node.#name_visit(&path.add(key.clone()), index, f);
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

fn gen_group_from_node_impls<'a, IT>(group: &'a Ident,
                                    nodes: IT)
                                    -> impl Iterator<Item = Tokens> + 'a
    where IT: Iterator<Item = &'a Node> + 'a
{
    once(quote!{
        impl <WD> ::std::convert::From<WD> for #group
            where WD: ::vtree::widget::WidgetDataTrait<#group> + 'static
        {
            fn from(widget_data: WD) -> #group {
                #group::Widget(Box::new(widget_data))
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

fn gen_all_nodes_from_group_impls<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
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

fn gen_builders<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
    pd.normal_nodes().map(move |node| {
        let name = &node.name;
        let name_str = node.name.as_ref();


        let maybe_params_field = node.params_ty.as_ref().map(|params_ty| {
            quote!{
                params: ::std::option::Option<#params_ty>,
            }
        });

        let maybe_child_field = node.child.as_ref().map(move |&(ty, ref child)| {
            let child_name = match child {
                &Child::Node(ref name) => {
                    match pd.node_by_name(name) {
                        Some(&Node::Normal(..)) => quote!{super::#name},
                        Some(&Node::Text) => quote!{::std::borrow::Cow<'static, str>},
                        None => unreachable!(),
                    }
                }
                &Child::Group(ref name) => quote!{super::groups::#name},
            };
            let ty = match ty {
                ChildType::Single => {
                    quote!{
                        ::vtree::child::Single<#child_name, super::groups::AllNodes>,
                    }
                }
                ChildType::Optional => {
                    quote!{
                        ::vtree::child::Option<#child_name, super::groups::AllNodes>,
                    }
                }
                ChildType::Multi => {
                    quote!{
                        ::vtree::child::Multi<#child_name, super::groups::AllNodes>,
                    }
                }
            };
            quote!{
                children: ::std::option::Option<#ty>,
            }
        });


        let maybe_params_constr = node.params_ty.as_ref().map(|_| {
            quote!{
                params: None,
            }
        });

        let maybe_child_constr = node.child.as_ref().map(|_| {
            quote!{
                children: None,
            }
        });


        let maybe_params_build_arg = node.params_ty.as_ref().map(|_| {
            let maybe_comma = node.child.as_ref().map(|_| quote!{,});

            quote!{
                self.params.unwrap_or_default()
                #maybe_comma
            }
        });

        let maybe_child_build_arg = node.child.as_ref().map(|&(ty, _)| {
            match ty {
                ChildType::Single => {
                    let err = format!("Builder: children not set for `{}`", name_str);
                    quote!{
                        self.children.expect(#err),
                    }
                }
                ChildType::Optional | ChildType::Multi => {
                    quote!{
                        self.children.unwrap_or_default(),
                    }
                }
            }
        });


        let maybe_params_builder_setter_impl = node.params_ty.as_ref().map(|params_ty| {
            quote!{
                impl ::vtree::node::BuilderSetter<::vtree::node::BuilderParams, #params_ty> for #name {
                    fn builder_set(&mut self, value: #params_ty) {
                        assert!(self.params.is_none(), "Params already set");
                        self.params = Some(value);
                    }
                }
            }
        });

        let maybe_child_builder_setter_impl = node.child.as_ref().map(|&(ty, ref child)| {
            let child_name = match child {
                &Child::Node(ref name) => {
                    match pd.node_by_name(name) {
                        Some(&Node::Normal(..)) => quote!{super::#name},
                        Some(&Node::Text) => quote!{::std::borrow::Cow<'static, str>},
                        None => unreachable!(),
                    }
                }
                &Child::Group(ref name) => quote!{super::groups::#name},
            };
            let child_ty = match ty {
                ChildType::Single => {
                    quote!{
                        ::vtree::child::Single<#child_name, super::groups::AllNodes>,
                    }
                }
                ChildType::Optional => {
                    quote!{
                        ::vtree::child::Option<#child_name, super::groups::AllNodes>,
                    }
                }
                ChildType::Multi => {
                    quote!{
                        ::vtree::child::Multi<#child_name, super::groups::AllNodes>,
                    }
                }
            };
            quote!{
                impl ::vtree::node::BuilderSetter<::vtree::node::BuilderChild, #child_ty> for #name {
                    fn builder_set(&mut self, value: #child_ty) {
                        assert!(self.children.is_none(), "Child already set");
                        self.children = Some(value);
                    }
                }
            }
        });

        let maybe_params_fn = node.params_ty.as_ref().map(|params_ty| {
            quote!{
                pub fn set_params(mut self, params: #params_ty) -> #name {
                    assert!(self.params.is_none(), "Params already set");
                    self.params = Some(params);
                    self
                }

                pub fn params(self) -> <#params_ty as ::vtree::node::Params<#name>>::Builder {
                    ::vtree::node::Params::builder(self)
                }
            }
        });

        let maybe_child_fn = node.child.as_ref().map(|&(ty, ref child)| {
            let child_name = match child {
                &Child::Node(ref name) => {
                    match pd.node_by_name(name) {
                        Some(&Node::Normal(..)) => quote!{super::#name},
                        Some(&Node::Text) => quote!{::std::borrow::Cow<'static, str>},
                        None => unreachable!(),
                    }
                }
                &Child::Group(ref name) => quote!{super::groups::#name},
            };
            let child_ty = match ty {
                ChildType::Single => {
                    quote!{
                        ::vtree::child::Single<#child_name, super::groups::AllNodes>,
                    }
                }
                ChildType::Optional => {
                    quote!{
                        ::vtree::child::Option<#child_name, super::groups::AllNodes>,
                    }
                }
                ChildType::Multi => {
                    quote!{
                        ::vtree::child::Multi<#child_name, super::groups::AllNodes>,
                    }
                }
            };
            let children_builder = match ty {
                ChildType::Single => {
                    quote!{
                        ::vtree::child::SingleBuilder
                    }
                }
                ChildType::Optional => {
                    quote!{
                        ::vtree::child::OptionBuilder
                    }
                }
                ChildType::Multi => {
                    quote!{
                        ::vtree::child::MultiBuilder
                    }
                }
            };
            quote!{
                pub fn set_children(mut self, children: #child_ty) -> #name {
                    assert!(self.children.is_none(), "Children already set");
                    self.children = Some(children);
                    self
                }

                pub fn children(self) -> #children_builder<#name, #child_name, super::groups::AllNodes> {
                    #children_builder::new(self)
                }
            }
        });

        quote!{
            pub struct #name {
                #maybe_params_field
                #maybe_child_field
            }

            impl #name {
                pub fn new() -> #name {
                    #name {
                        #maybe_params_constr
                        #maybe_child_constr
                    }
                }

                pub fn build(self) -> super::#name {
                    super::#name::new(
                        #maybe_params_build_arg
                        #maybe_child_build_arg
                    )
                }

                #maybe_params_fn
                #maybe_child_fn
            }

            #maybe_params_builder_setter_impl
            #maybe_child_builder_setter_impl
        }
    })
}

pub fn generate_defs(pd: ParsedData) -> String {
    let all_nodes_ident = Ident::new("AllNodes");
    let node_defs = gen_node_defs(&pd);
    let group_defs = pd.groups_nodes()
        .map(|(name, nodes)| gen_group_def(name, nodes))
        .chain(once(gen_group_def(&all_nodes_ident, pd.nodes())));
    let all_nodes_impl = gen_all_nodes_impl(&pd);
    let group_from_node_impls = pd.groups_nodes()
        .flat_map(|(name, nodes)| gen_group_from_node_impls(name, nodes))
        .chain(gen_group_from_node_impls(&all_nodes_ident, pd.nodes()));
    let all_nodes_from_group_impls = gen_all_nodes_from_group_impls(&pd);
    let builders = gen_builders(&pd);
    let defs = quote!{
        #(#node_defs)*
        pub mod groups {
            #(#group_defs)*
            #all_nodes_impl
            #(#group_from_node_impls)*
            #(#all_nodes_from_group_impls)*
        }
        pub mod builders {
            #(#builders)*
        }
    };
    println!("{}", defs);
    defs.into_string()
}
