use syn::{StrLit, IntLit, ByteStrLit};
use syn::parse::{ident, path, int, string, byte_string, expr, boolean};
use quote::Tokens;
use quote::ToTokens;

#[derive(Debug)]
pub enum Value {
    String(String),
    Int(u64),
    Bytes(Vec<u8>),
    Expr(String),
    Bool(bool),
}

#[derive(Debug)]
pub enum TextValue {
    String(String),
    Expr(bool, String),
}

#[derive(Debug)]
pub enum Params {
    KeyValue(Vec<(String, Value)>),
    Whole(Value),
}

#[derive(Debug)]
pub enum Node {
    Node {
        name: String,
        key: Option<Value>,
        params: Params,
        children: Vec<Node>,
    },
    Text {
        value: TextValue,
        key: Option<Value>,
    },
}

named!(parse_expr -> String,
    do_parse!(
        punct!("(") >>
        e: expr >>
        punct!(")") >>
        ({
            let mut t = Tokens::new();
            e.to_tokens(&mut t);
            t.into_string()
        })
    )
);

named!(parse_value -> Value,
    alt!(
        string => {|v: StrLit| Value::String(v.value)}
        |
        int => {|v: IntLit| Value::Int(v.value)} // TODO: negative ints
        |
        parse_expr => {|e| Value::Expr(e)}
        |
        boolean => {|v| Value::Bool(v)}
        |
        byte_string => {|v: ByteStrLit| Value::Bytes(v.value)}
    )
);

named!(pub parse_node -> Node,
    alt!(
        do_parse!(
            value: alt!(
                string => {|v: StrLit| TextValue::String(v.value)}
                |
                tuple!(option!(punct!("+")), parse_expr) =>
                    {|(add, e): (Option<_>, _)| TextValue::Expr(add.is_some(), e)}
            ) >>
            key: option!(preceded!(punct!("@"), parse_value)) >>
            (
                Node::Text {
                    value: value,
                    key: key,
                }
            )
        )
        |
        do_parse!(
            name: path >>
            key: option!(preceded!(punct!("@"), parse_value)) >>
            params: alt!(
                do_parse!(
                    value: preceded!(punct!("="), parse_value) >>
                    (Params::Whole(value))
                )
                |
                do_parse!(
                    kvs: many0!(do_parse!(
                        name: ident >>
                        value: alt!(
                            preceded!(punct!("="), parse_value)
                            |
                            punct!("?") => {|_| Value::Bool(true)}
                        ) >>
                        ((name.to_string(), value))
                    )) >>
                    (Params::KeyValue(kvs))
                )
            ) >>
            children: alt!(
                punct!("/") => {|_| vec![]}
                |
                parse_node => {|n| vec![n]}
                |
                do_parse!(
                    punct!("{") >>
                    children: many0!(parse_node) >>
                    punct!("}") >>
                    (children)
                )
            ) >>
            ({
                let mut name_token = Tokens::new();
                name.to_tokens(&mut name_token);
                Node::Node {
                    name: name_token.into_string(),
                    key: key,
                    params: params,
                    children: children,
                }
            })
        )
    )
);
