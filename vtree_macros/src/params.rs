use quote::Tokens;
use syn::Ident;
use syn::{parse_derive_input, Body, VariantData, Field, Attribute, MetaItem};

fn field_contains_event_attr(field: &Field) -> bool {
    field.attrs.iter().any(|attr| attr.name() == "event")
}

fn gen_builder(name: &Ident, fields: &[Field]) -> Tokens {
    let builder_name: Ident = format!("{}Builder", name.as_ref()).into();

    let struct_fields = fields
        .into_iter()
        .filter(|f| !field_contains_event_attr(f))
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let ty = &field.ty;
            quote! {
                #field_name: ::std::option::Option<#ty>,
            }
        });

    let build_fields = fields
        .into_iter()
        .filter(|f| !field_contains_event_attr(f))
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let unwrap = field.attrs
                .iter()
                .filter_map(|a| match a {
                    &Attribute {
                        value: MetaItem::NameValue(ref id, ref lit),
                        ..
                    } if id == "default" => Some(lit),
                    _ => None,
                })
                .next();
            let unwrap = match unwrap {
                Some(lit) => quote!{
                    unwrap_or(#lit.into())
                },
                None => quote!{
                    unwrap_or_default()
                },
            };
            quote! {
                #field_name: self.#field_name.#unwrap,
            }
        });

    let setters_getters = fields
        .into_iter()
        .filter(|f| !field_contains_event_attr(f))
        .map(|field| {
            let ty = &field.ty;
            let field_name = field.ident.as_ref().unwrap();
            let setter_name: Ident = format!("set_{}", field_name.as_ref()).into();
            let mut_getter_name: Ident = format!("mut_{}", field_name.as_ref()).into();
            quote! {
                pub fn #setter_name(mut self, value: #ty) -> #builder_name<PB> {
                    self.#field_name = Some(value);
                    self
                }

                pub fn #field_name(&self) -> ::std::option::Option<&#ty> {
                    self.#field_name.as_ref()
                }

                pub fn #mut_getter_name(&mut self) -> ::std::option::Option<&mut #ty> {
                    self.#field_name.as_mut()
                }
            }
        });

    let constructor_fields = fields
        .into_iter()
        .filter(|f| !field_contains_event_attr(f))
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            quote! {
                #field_name: None,
            }
        });

    quote!{
        impl <PB> ::vtree::node::Params<PB> for #name
            where PB: ::vtree::node::BuilderSetter<::vtree::node::BuilderParams, #name>
        {
            type Builder = #builder_name<PB>;

            fn builder(parent_builder: PB) -> #builder_name<PB> {
                #builder_name::new(parent_builder)
            }
        }

        pub struct #builder_name<PB> {
            parent_builder_: PB,
            #(#struct_fields)*
        }

        impl <PB> #builder_name<PB>
            where PB: ::vtree::node::BuilderSetter<::vtree::node::BuilderParams, #name>
        {
            pub fn new(parent_builder: PB) -> #builder_name<PB> {
                #builder_name {
                    parent_builder_: parent_builder,
                    #(#constructor_fields)*
                }
            }

            pub fn build(self) -> PB {
                let mut pb = self.parent_builder_;
                pb.builder_set(#name {
                    #(#build_fields)*
                });
                pb
            }
            #(#setters_getters)*
        }
    }
}

pub fn handle_params(input: String) -> String {
    let mut ast = parse_derive_input(&input).unwrap();
    let builder = {
        let mut fields = match ast.body {
            Body::Struct(VariantData::Struct(ref mut fields)) => fields,
            Body::Struct(_) => panic!("params macro: units and tuples not supported"),
            Body::Enum(_) => panic!("params macro: enums not supported"),
        };
        let builder = gen_builder(&ast.ident, &fields);
        for field in fields.iter_mut() {
            field.attrs.retain(|a| !["default", "event"].contains(&a.name()));
        }
        builder
    };

    quote!(
        #ast
        #builder
    ).into_string()
}
