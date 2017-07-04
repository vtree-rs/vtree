use quote::Tokens;
use parser::{ParsedData, ChildType, Node, NodeNormal, Child};

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

pub fn gen_node_defs<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
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
