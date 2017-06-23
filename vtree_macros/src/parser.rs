use syn::{Ident, Path};
use std::collections::HashMap;
use std::collections::HashSet;
use parser::parser::{GroupOp};

#[derive(Debug, Clone, Copy)]
pub enum ChildType {
    Single,
    Optional,
    Multi,
}

#[derive(Debug, Clone)]
pub enum Child {
    Node(Ident),
    Group(Ident),
}

#[derive(Debug, Clone)]
pub struct NodeNormal {
    pub name: Ident,
    pub params_ty: Option<Path>,
    pub child: Option<(ChildType, Child)>,
}

#[derive(Debug, Clone)]
pub enum Node {
    Normal(NodeNormal),
    Text,
}

impl Node {
    pub fn normal(&self) -> Option<&NodeNormal> {
        match self {
            &Node::Normal(ref n) => Some(n),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: Ident,
    pub nodes: HashSet<Ident>,
}

#[derive(Debug, Clone)]
pub struct ParsedData {
    node_by_name: HashMap<Ident, Node>,
    groups: Vec<Group>,
}

impl ParsedData {
    pub fn nodes<'a>(&'a self) -> impl Iterator<Item = &'a Node> + 'a {
        self.node_by_name.values()
    }

    pub fn normal_nodes<'a>(&'a self) -> impl Iterator<Item = &'a NodeNormal> + 'a {
        self.node_by_name.values().filter_map(Node::normal)
    }

    pub fn groups_nodes<'a>(&'a self) -> impl Iterator<Item = (&'a Ident, impl Iterator<Item = &'a Node> + 'a)> + 'a {
        self.groups.iter().map(move |group| {
            let it = group.nodes.iter().filter_map(move |name| self.node_by_name(name));
            (&group.name, it)
        })
    }

    pub fn node_by_name<'a>(&'a self, name: &Ident) -> Option<&'a Node> {
        self.node_by_name.get(name)
    }
}

fn resolve_groups(
    groups: &HashMap<Ident, Vec<(GroupOp, Child)>>,
    children: &[(GroupOp, Child)]
) -> HashSet<Ident> {
    let mut set = HashSet::new();
    set.insert(Ident::new("Text"));
    for &(op, ref child) in children {
        match child {
            &Child::Node(ref id) => {
                match op {
                    GroupOp::Add => {
                        set.insert(id.clone());
                    }
                    GroupOp::Sub => {
                        set.remove(id);
                    }
                }
            }
            &Child::Group(ref id) => {
                for child in resolve_groups(groups, &groups.get(id).unwrap()[..]) {
                    match op {
                        GroupOp::Add => {
                            set.insert(child);
                        }
                        GroupOp::Sub => {
                            set.remove(&child);
                        }
                    }
                }
            }
        }
    }
    set
}

pub fn parse(input: &str) -> ParsedData {
    // TODO: detect loops
    // TODO: report errors

    let (nodes, groups) = parser::parse(input).expect("vtree define_nodes");

    let mut nodes_by_name: HashMap<_, _> = nodes
        .iter()
        .cloned()
        .map(|n| (n.name.clone(), Node::Normal(n)))
        .collect();

    let groups: HashMap<Ident, Vec<(GroupOp, Child)>> = groups
        .into_iter()
        .map(|g| (g.name, g.children))
        .collect();

    let groups: Vec<Group> = groups
        .iter()
        .map(|(id, children)| Group {
            name: id.clone(),
            nodes: resolve_groups(&groups, children).into_iter().collect(),
        })
        .collect();

    let text_ident = Ident::new("Text");
    if groups.iter().any(|group| group.nodes.contains(&text_ident)) {
        nodes_by_name.insert(text_ident, Node::Text);
    }

    ParsedData {
        node_by_name: nodes_by_name,
        groups: groups,
    }
}

mod parser {
    use parser::{Child, NodeNormal, ChildType};
    use syn::parse::{ident, path};
    use syn::Ident;

    #[derive(Debug, Clone, Copy)]
    pub enum GroupOp {
        Add,
        Sub,
    }

    #[derive(Debug, Clone)]
    pub struct Group {
        pub name: Ident,
        pub children: Vec<(GroupOp, Child)>,
    }

    named!(parse_child -> Child,
        alt!(
            preceded!(punct!("@"), ident) => {|g| Child::Group(g)}
            |
            ident => {|n| Child::Node(n)}
        )
    );

    named!(parse_nodes -> Vec<NodeNormal>,
        terminated_list!(punct!(","), do_parse!(
            name: ident >>
            params_ty: option!(delimited!(punct!("<"), path, punct!(">"))) >>
            child: option!(do_parse!(
                punct!(":") >>
                child_ty: alt!(
                    keyword!("mul") => {|_| ChildType::Multi}
                    |
                    keyword!("opt") => {|_| ChildType::Optional}
                    |
                    epsilon!() => {|_| ChildType::Single}
                ) >>
                child: parse_child >>
                (child_ty, child)
            )) >>
            (NodeNormal {
                name: name,
                params_ty: params_ty,
                child: child,
            })
        ))
    );

    named!(parse_groups -> Vec<Group>,
        terminated_list!(punct!(","), do_parse!(
            name: ident >>
            punct!(":") >>
            children: many0!(alt!(
                preceded!(option!(punct!("+")), parse_child) => {|c| (GroupOp::Add, c)}
                |
                preceded!(punct!("-"), parse_child) => {|c| (GroupOp::Sub, c)}
            )) >>
            (Group {
                name: name,
                children: children,
            })
        ))
    );

    named!(pub parse -> (Vec<NodeNormal>, Vec<Group>),
        do_parse!(
            keyword!("nodes") >>
            punct!("{") >>
            nodes: parse_nodes >>
            punct!("}") >>
            keyword!("groups") >>
            punct!("{") >>
            groups: parse_groups >>
            punct!("}") >>
            (nodes, groups)
        )
    );
}
