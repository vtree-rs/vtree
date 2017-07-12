use quote::Tokens;
use syn::Ident;
use syn::{self, parse_derive_input, Body, VariantData, Field, Attribute, MetaItem, Lit};

fn field_is_event(field: &Field) -> bool {
    field.attrs.iter().any(|attr| attr.name() == "event")
}

fn gen_builder(name: &Ident, fields: &[Field]) -> Tokens {
    let builder_name: Ident = format!("{}Builder", name.as_ref()).into();
    let events_name: Ident = format!("{}Events", name.as_ref()).into();
    let events_name_str = events_name.as_ref();
    let has_events = fields.into_iter().any(|f| !field_is_event(f));

    let struct_fields = fields
        .into_iter()
        .filter(|f| !field_is_event(f))
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let ty = &field.ty;
            quote! {
                #field_name: ::std::option::Option<#ty>,
            }
        });

    let build_fields = fields
        .into_iter()
        .filter(|f| !field_is_event(f))
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
        .map(|field| {
            let ty = &field.ty;
            let field_name = field.ident.as_ref().unwrap();
            let setter_name: Ident = format!("set_{}", field_name.as_ref()).into();
            let mut_getter_name: Ident = format!("mut_{}", field_name.as_ref()).into();

            if field_is_event(field) {
                quote! {
                    pub fn #setter_name<F>(mut self, f: F) -> #builder_name<PB>
                        where F: Fn(#ty) + 'static
                    {
                        match self.events_ {
                            ::std::option::Option::Some(ref mut events) => events,
                            ::std::option::Option::None => {
                                self.events_ =
                                    ::std::option::Option::Some(::std::default::Default::default());
                                self.events_.as_mut().unwrap()
                            }
                        }.#field_name = ::std::option::Option::Some(::std::rc::Rc::new(f));
                        self
                    }
                }
            } else {
                quote! {
                    pub fn #setter_name(mut self, value: #ty) -> #builder_name<PB> {
                        self.#field_name = ::std::option::Option::Some(value);
                        self
                    }

                    pub fn #field_name(&self) -> ::std::option::Option<&#ty> {
                        self.#field_name.as_ref()
                    }

                    pub fn #mut_getter_name(&mut self) -> ::std::option::Option<&mut #ty> {
                        self.#field_name.as_mut()
                    }
                }
            }
        });

    let constructor_fields = fields
        .into_iter()
        .filter(|f| !field_is_event(f))
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            quote! {
                #field_name: ::std::option::Option::None,
            }
        });

    let maybe_events = if has_events {
        let event_fields = fields
            .into_iter()
            .filter(|f| field_is_event(f))
            .map(|field| {
                let field_name = field.ident.as_ref().unwrap();
                let ty = &field.ty;
                quote! {
                    #field_name: ::std::option::Option<::std::rc::Rc<Fn(#ty) + 'static>>,
                }
            });

        let has_event_variants = fields
            .into_iter()
            .filter(|f| field_is_event(f))
            .map(|field| {
                let field_name = field.ident.as_ref().unwrap();
                let field_name_str = field_name.as_ref();
                quote! {
                    #field_name_str => self.#field_name.is_some(),
                }
            });

        let send_event_variants = fields
            .into_iter()
            .filter_map(|field| {
                let ty: Ident = match field.attrs.iter().find(|attr| attr.name() == "event") {
                    Some(&Attribute {
                        value: MetaItem::NameValue(_, Lit::Str(ref ty, _)),
                        ..
                    }) => ty.as_str().into(),
                    Some(_) => panic!("`event` attribute has to hold a string"),
                    None => return None,
                };

                let field_name = field.ident.as_ref().unwrap();
                let field_name_str = field_name.as_ref();
                Some(quote! {
                    (#field_name_str, AllEvent::#ty(e)) => {
                        if let ::std::option::Option::Some(ref mut h) = self.#field_name {
                            h(e);
                        }
                    }
                })
            });

        Some(quote!{
            #[derive(Default, Clone)]
            pub struct #events_name {
                #(#event_fields)*
            }

            impl ::vtree::node::ParamsEvents<AllEvent> for #events_name {
                fn has(&self, event_name: &str) -> bool {
                    match event_name {
                        #(#has_event_variants)*
                        event_name => panic!("unsupported event name `{}`", event_name),
                    }
                }

                fn send(&mut self, event_name: &str, event: AllEvent) {
                    match (event_name, event) {
                        #(#send_event_variants)*
                        (event_name, event) => {
                            panic!(
                                "unsupported event name `{}` and event combination: {:#?}",
                                event_name,
                                event
                            )
                        }
                    }
                }
            }

            impl ::std::fmt::Debug for #events_name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    f.write_str(#events_name_str)
                }
            }

        })
    } else {
        None
    };

    let maybe_struct_events_field = if has_events {
        Some(quote!{
            events_: ::std::option::Option<#events_name>,
        })
    } else {
        None
    };

    let maybe_constructor_events_field = if has_events {
        Some(quote!{
            events_: ::std::option::Option::None,
        })
    } else {
        None
    };

    let maybe_build_events_field = if has_events {
        Some(quote!{
            events_: ::vtree::node::ParamsEventsWrapper(self.events_),
        })
    } else {
        None
    };

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
            #maybe_struct_events_field
            #(#struct_fields)*
        }

        impl <PB> #builder_name<PB>
            where PB: ::vtree::node::BuilderSetter<::vtree::node::BuilderParams, #name>
        {
            pub fn new(parent_builder: PB) -> #builder_name<PB> {
                #builder_name {
                    parent_builder_: parent_builder,
                    #maybe_constructor_events_field
                    #(#constructor_fields)*
                }
            }

            pub fn build(self) -> PB {
                let mut pb = self.parent_builder_;
                pb.builder_set(#name {
                    #(#build_fields)*
                    #maybe_build_events_field
                });
                pb
            }
            #(#setters_getters)*
        }

        #maybe_events
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
        let has_events = fields.into_iter().any(|f| !field_is_event(f));
        fields.retain(|f| !field_is_event(f));
        if has_events {
            let events_name: Ident = format!("{}Events", ast.ident.as_ref()).into();
            fields.push(Field {
                ident: Some("events_".into()),
                vis: syn::Visibility::Inherited,
                attrs: vec![],
                ty: syn::Ty::Path(
                    None,
                    syn::parse_path(&quote!(::vtree::node::ParamsEventsWrapper<#events_name>).into_string()).unwrap()
                ),
            });
        }
        for field in fields.iter_mut() {
            field.attrs.retain(|a| !["default"].contains(&a.name()));
        }
        builder
    };

    quote!(
        #ast
        #builder
    ).into_string()
}
