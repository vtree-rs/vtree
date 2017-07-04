#![feature(plugin)]
#![feature(proc_macro)]

extern crate vtree;
extern crate vtree_macros;
extern crate vtree_markup;

use vtree::key::Key;
use vtree::widget::{Widget, WidgetData};
use vtree::diff::{self, Context, Differ, Path};
use vtree::node;
use vtree_macros::define_nodes;
use vtree_markup::markup;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AParams {
    s: String,
}

impl <PB> node::Params<PB> for AParams
    where PB: node::BuilderSetter<node::BuilderParams, AParams>
{
    type Builder = AParamsBuilder<PB>;

    fn builder(parent_builder: PB) -> AParamsBuilder<PB> {
        AParamsBuilder {parent_builder: parent_builder, s: None}
    }
}

pub struct AParamsBuilder<PB> {
    parent_builder: PB,
    s: Option<String>,
}

impl <PB> AParamsBuilder<PB>
    where PB: node::BuilderSetter<node::BuilderParams, AParams>
{
    pub fn build(self) -> PB {
        let mut pb = self.parent_builder;
        pb.builder_set(AParams {s: self.s.unwrap_or_default()});
        pb
    }

    pub fn set_s(mut self, value: String) -> Self {
        self.s = Some(value);
        self
    }
}

// struct A
// struct Text
// struct Widget
//
// mod children {
//     enum A {}
// }
//
// mod builders {
//     struct A
// }

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

// #[derive(Debug, Clone)]
// struct GroupAWidget;
// impl Widget for GroupAWidget {
//     type Input = String;
//     type Output = GroupA;
//
//     fn new() -> Self {
//         GroupAWidget
//     }
//
//     fn render(&self, i: Self::Input) -> Option<GroupA> {
//         Some(a(AParams { s: i }, &[]))
//     }
// }

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
