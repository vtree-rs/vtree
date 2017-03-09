use syn::parse::{ident, path};
use syn::{Ident, Path};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum NodeChildType {
    Single,
    Optional,
    Multi,
}

#[derive(Debug, Clone)]
pub struct NodeChild {
    pub name: Ident,
    pub group: Ident,
    pub child_type: NodeChildType,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: Ident,
    pub params_type: Option<Path>,
    pub fields: Vec<NodeChild>,
}

#[derive(Debug, Clone)]
pub struct ParsedData {
    pub nodes: Vec<Node>,
    pub group_name_to_nodes: HashMap<Ident, Vec<Node>>,
}

named!(parse -> Vec<(Ident, Option<Path>, Option<Ident>, Vec<(Ident, NodeChildType, Ident)>)>,
    terminated_list!(punct!(","), tuple!(
        ident,
        option!(delimited!(punct!("<"), path, punct!(">"))),
        option!(do_parse!(
            punct!(":") >>
            name: ident >>
            (name)
        )),
        opt_vec!(delimited!(
            punct!("{"),
            terminated_list!(punct!(","), do_parse!(
                name: ident >>
                punct!(":") >>
                field_ty: alt!(
                    keyword!("mul") => {|_| NodeChildType::Multi}
                    |
                    keyword!("opt") => {|_| NodeChildType::Optional}
                    |
                    epsilon!() => {|_| NodeChildType::Single}
                ) >>
                ty_name: ident >>
                (name, field_ty, ty_name)
            )),
            punct!("}")
        ))
    ))
);

pub fn parse_nodes(input: &str) -> ParsedData {
    let mut nodes = Vec::new();
    let mut group_name_to_nodes = HashMap::new();
    for (name, params_type, group, fields) in parse(input).expect("unable to parse vtree-nodes") {
        let fields = fields
            .into_iter()
            .map(|(f_name, f_ty, f_group)| {
                NodeChild {
                    name: f_name,
                    group: f_group,
                    child_type: f_ty,
                }
            })
            .collect();
        let node = Node {
            name: name,
            params_type: params_type,
            fields: fields,
        };
        if let Some(group) = group {
            group_name_to_nodes.entry(group).or_insert_with(|| Vec::new()).push(node.clone());
        }
        nodes.push(node);
    }
    ParsedData {
        nodes: nodes,
        group_name_to_nodes: group_name_to_nodes,
    }
}
