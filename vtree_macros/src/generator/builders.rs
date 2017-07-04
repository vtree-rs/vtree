use quote::Tokens;
use parser::{ParsedData, ChildType, Node, Child};

pub fn gen_builders<'a>(pd: &'a ParsedData) -> impl Iterator<Item = Tokens> + 'a {
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
