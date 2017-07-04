mod builders;
mod groups;
mod nodes;

use self::builders::*;
use self::groups::*;
use self::nodes::*;

use syn::Ident;
use parser::ParsedData;
use std::iter::once;

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
