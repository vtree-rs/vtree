#![feature(plugin)]
#![feature(proc_macro)]

extern crate vtree;
#[macro_use]
extern crate vtree_macros;
extern crate vtree_markup;

use vtree::diff::{self, Context, Differ, Path};
use vtree::node;
use vtree_macros::{define_nodes, define_params};
use vtree_markup::markup;

define_params!{
    #[derive(Default, Debug, Clone, PartialEq)]
    pub struct AParams {
        s: String,
    }
}

define_nodes!{
    nodes {
        A<::AParams>: mul @Foo,
        Label: mul Text,
        B,
        C,
    }
    groups {
        Bar: A B,
        Foo: @Bar C,
    }
}

use groups::AllNodes;

#[derive(Debug)]
struct MyDiffer;
impl Differ<AllNodes> for MyDiffer {
    fn diff_added(&self, path: &Path, index: usize, curr: &AllNodes) {
        println!("diff_added");
    }

    fn diff_removed(&self, path: &Path, index: usize, last: &AllNodes) {
        println!("diff_removed");
    }

    fn diff_replaced(&self, path: &Path, index: usize, curr: &AllNodes, last: &AllNodes) {
        println!("diff_replaced");
    }

    fn diff_params_changed(&self, path: &Path, curr: &AllNodes, last: &AllNodes) {
        println!("diff_params_changed");
    }

    fn diff_reordered<I: Iterator<Item=(usize, usize)>>(&self, path: &Path, indices: I) {
        println!("diff_reordered");
    }
}

fn main() {
    let mut test_a = markup!(
        A s="node1" A s="node2" "asd"
    );

    let mut test_b = markup!(
        A s="node2" /
    );

    let ctx = Context::new();
    let path = diff::Path::new();
    AllNodes::expand_widgets(&mut test_a, None, &path);
    AllNodes::expand_widgets(&mut test_b, None, &path);
    AllNodes::diff(&test_b, &test_a, &path, 0, &ctx, &MyDiffer);
}
