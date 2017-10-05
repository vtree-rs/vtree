use parser::{Node, TextValue, Value, Params};
use quote::{Tokens, Ident};

fn render_param_value(val: &Value) -> Tokens {
    match val {
        &Value::String(ref v) => quote!{#v},
        &Value::Int(ref v) => quote!{#v},
        &Value::Bytes(ref v) => quote!{#v},
        &Value::Expr(ref v) => {
            let v = Ident::new(v.to_string());
            quote!{(#v)}
        }
        &Value::Bool(ref v) => quote!{#v},
    }
}


fn render_value(val: &Value) -> Tokens {
    match val {
        &Value::String(ref v) => quote!{#v},
        &Value::Int(ref v) => quote!{#v},
        &Value::Bytes(ref v) => quote!{#v},
        &Value::Expr(ref v) => {
            let v = Ident::new(v.to_string());
            quote!{(#v)}
        }
        &Value::Bool(ref v) => quote!{#v},
    }
}

pub fn render_node(node: Node) -> Tokens {
    match node {
        Node::Node {name, params, children, ..} => {
            let name = Ident::new(name);

            let maybe_params = match params {
                Params::KeyValue(kvs) => {
                    if kvs.is_empty() {
                        None
                    } else {
                        let kvs = kvs.into_iter().map(|(key, val)| {
                            let set_key = Ident::new(format!("set_{}", key));
                            let val = render_param_value(&val);
                            quote!{
                                .#set_key(#val.into())
                            }
                        });

                        Some(quote!{
                            .params()
                            #(#kvs)*
                            .build()
                        })
                    }
                }
                Params::Whole(value) => {
                    let val = render_param_value(&value);
                    Some(quote!{
                        .set_params(#val.into())
                    })
                }
            };

            let maybe_children = if children.is_empty() {
                None
            } else {
                let children = children.into_iter().enumerate().map(|(index, child)| {
                    match &child {
                        &Node::Text{value: TextValue::Expr(true, _), ..} => {
                            let child_rendered = render_node(child);
                            quote!{
                                .add_all(#child_rendered)
                            }
                        }
                        _ => {
                            let key = {
                                let key = match child {
                                    Node::Node {ref key, ..} => key,
                                    Node::Text {ref key, ..} => key,
                                };
                                render_value(key
                                        .as_ref()
                                        .unwrap_or(&Value::Int(u64::max_value() - index as u64)))
                            };
                            let child_rendered = render_node(child);
                            quote!{
                                .add(#key.into(), #child_rendered)
                            }
                        }
                    }
                });

                Some(quote!{
                    .children()
                    #(#children)*
                    .build()
                })
            };

            quote!{
                #name::builder()
                    #maybe_params
                    #maybe_children
                    .build()
                    .into()
            }
        },
        Node::Text {value, ..} => {
            match value {
                TextValue::String(v) => quote!{#v.into()},
                TextValue::Expr(_, v) => {
                    let v = Ident::new(v);
                    quote!{(#v).into()}
                }
            }
        }
    }
}
