use parser::{Node, TextValue, Value};
use quote::{Tokens, Ident};

fn render_param_value(val: &Value) -> Tokens {
    match val {
        &Value::String(ref v) => quote!{::vtree::markup::ParamValue::from(#v)},
        &Value::Int(ref v) => quote!{::vtree::markup::ParamValue::Int(#v)},
        &Value::Bytes(ref v) => quote!{::vtree::markup::ParamValue::from(#v)},
        &Value::Expr(ref v) => {
            let v = Ident::new(v.to_string());
            quote!{::vtree::markup::ParamValue::from(#v)}
        }
        &Value::Bool(ref v) => quote!{::vtree::markup::ParamValue::Bool(#v)},
    }
}


fn render_value(val: &Value) -> Tokens {
    match val {
        &Value::String(ref v) => quote!{#v},
        &Value::Int(ref v) => quote!{#v},
        &Value::Bytes(ref v) => quote!{#v},
        &Value::Expr(ref v) => {
            let v = Ident::new(v.to_string());
            quote!{#v}
        }
        &Value::Bool(ref v) => quote!{#v},
    }
}

pub fn render_node(node: Node) -> Tokens {
    match node {
        Node::Node {name, params, children, ..} => {
            let name = Ident::new(name);

            let params = params.into_iter().map(|(key, val)| {
                let val = render_param_value(&val);
                quote!{
                    (#key, #val),
                }
            });

            let children = children.into_iter().enumerate().map(|(index, child)| {
                let key = {
                    let key = match child {
                        Node::Node {ref key, ..} => key,
                        Node::Text {ref key, ..} => key,
                    };
                    render_value(key.as_ref().unwrap_or(&Value::Int(index as u64)))
                };
                let child = render_node(child);
                quote!{
                    (#key.into(), #child),
                }
            });

            quote!{
                #name(
                    vec![#(#params)*].iter(),
                    vec![#(#children)*].iter(),
                )
            }
        },
        Node::Text {value, ..} => {
            match value {
                TextValue::String(v) => quote!{#v.into()},
                TextValue::Expr(v) => {
                    let v = Ident::new(v);
                    quote!{#v.into()}
                }
            }
        }
    }
}
