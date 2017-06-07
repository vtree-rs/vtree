#![feature(plugin)]
#![feature(proc_macro)]

extern crate vtree;
extern crate vtree_macros;

use vtree::key::{Key, key};
use vtree::widget::{Widget, WidgetData};
use vtree::diff::{self, Context, Differ, Path};
use vtree_macros::define_nodes;

#[derive(Debug, Clone, PartialEq)]
pub struct AParams {
    s: String,
}

define_nodes!{
    A<AParams>: GroupA {
        child: mul GroupA,
    },
    Text<String>: GroupA,
}

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
    let mut test_a: AllNodes = a(
        AParams { s: "node1".to_string() },
        &[
            // (
            //     0,
            //     WidgetData::<GroupAWidget>("foo bar".to_string()).into()
            // ),

            (
                1,
                a(
                    AParams {
                        s: "node2".to_string(),
                    },
                    &[
                        (1, text("asd"))
                    ]
                )
            ),
        ]
    );

    let mut test_b: AllNodes = a(
        AParams { s: "node2".to_string() },
        &[
            (1, text("asd"))
            // (
            //     key(0),
            //     WidgetData::<GroupAWidget>("foo bar2".to_string()).into()
            // ),
        ]
    );

    let ctx = Context::new();
    let path = diff::Path::new();
    AllNodes::expand_widgets(&mut test_a, None, &path);
    AllNodes::expand_widgets(&mut test_b, None, &path);
    AllNodes::diff(&test_b, &test_a, &path, 0, &ctx, &MyDiffer);
}
