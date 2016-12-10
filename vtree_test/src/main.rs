#![feature(plugin)]
#![plugin(vtree_macros)]

extern crate vtree;

use vtree::key::{Key, key, KeyedDiff, KeyedNodes};
use vtree::widget::{Widget, WidgetData};
use vtree::diff::{self, Diff, Context};

#[derive(Debug, Clone, PartialEq)]
pub struct AParams {
    s: String,
}

define_nodes!(
    A<AParams>: GroupA {
        child: mul GroupA,
    },
);

#[derive(Debug, Clone)]
struct GroupAWidget;
impl Widget for GroupAWidget {
    type Input = String;
    type Output = GroupA;

    fn new() -> Self {
        GroupAWidget
    }

    fn render(&self, i: Self::Input) -> Option<Self::Output> {
        Some(a(AParams { s: i }, KeyedNodes::new()))
    }
}

struct MyDiffer;
impl Differ for MyDiffer {
    fn diff_group_a<'a>(&self, path: &diff::Path, curr: &GroupA, diff: diff::Diff) {
        println!("diff_group_a: `{}`: {:?}", path, diff);
    }

    fn reorder_a_child(&self,
                       path: &diff::Path,
                       parent: &A,
                       index_curr: usize,
                       index_last: usize) {
        println!("reorder_a_child: `{}`: {} => {}", path, index_last, index_curr);
    }

    fn params_changed_a(&self, path: &diff::Path, curr: &A, last: &A) {
        println!("params_changed_a: `{}`: {:?} => {:?}", path, last.params, curr.params);
    }
}

fn main() {
    let test_a: GroupA = a(
        AParams { s: "node1".to_string() },
        KeyedNodes::with_data(vec![
            (
                key(0),
                WidgetData::<GroupAWidget>("foo bar".to_string()).into()
            ),

            (
                key(1),
                a(
                    AParams {
                        s: "node2".to_string(),
                    },
                    KeyedNodes::new()
                )
            ),
        ]),
    );

    let test_b: GroupA = a(
        AParams { s: "node2".to_string() },
        KeyedNodes::with_data(vec![
            (
                key(0),
                WidgetData::<GroupAWidget>("foo bar2".to_string()).into()
            ),
        ]),
    );

    let ctx = Context::new(MyDiffer);
    let path = diff::Path::new();
    let test_a = test_a.expand_widgets(None, &path);
    let test_b = test_b.expand_widgets(Some(&test_a), &path);
    test_b.diff(&path, &test_a, &ctx);
}
