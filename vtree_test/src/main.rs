#![feature(plugin)]
#![plugin(vtree_macros)]

extern crate vtree;

use vtree::key::{Key, key, KeyedDiff, KeyedNodes};
use vtree::widget::{Widget, WidgetData};
use vtree::diff::{self, Diff};

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
		Some(GroupA::A(A {
			child: KeyedNodes::new(),
			params: AParams {
				s: i,
			}
		}))
	}
}

struct MyDiffer;
impl Differ for MyDiffer {
	fn diff_group_a<'a>(
		&self,
		path: &diff::Path,
		curr: &GroupA,
		diff: diff::Diff,
	) {
		println!("diff_group_a: `{}`: {:?}", path, diff);
	}

	fn reorder_a_child(
		&self,
		path: &diff::Path,
		index_curr: usize,
		index_last: usize,
	) {
		println!("reorder_a_child: `{}`: {} => {}", path, index_last, index_curr);
	}

	fn params_changed_a(&self, path: &diff::Path, curr: &A, last: &A) {
		println!("params_changed_a: `{}`: {:?} => {:?}", path, last.params, curr.params);
	}
}

pub struct Context<D: Differ> {
	// pub widgets: HashMap<diff::Path, Box<WidgetDataTrait<G>>>,
	pub differ: D,
}

impl <D: Differ> Context<D> {
	pub fn new(differ: D) -> Context<D> {
		Context {
			differ: differ,
		}
	}
}

fn main() {
	let test_a = GroupA::A(A {
		params: AParams {
			s: "node1".to_string(),
		},
		child: KeyedNodes::with_data(vec![
			(
				key(0),
				GroupA::Widget(Box::new(
					WidgetData::<GroupAWidget>("foo bar".to_string())
				)),
			),

			(
				key(1),
				GroupA::A(A {
					params: AParams {
						s: "node2".to_string(),
					},
					child: KeyedNodes::new(),
				}),
			),
		]),
	});

	let test_b = GroupA::A(A {
		params: AParams {
			s: "node2".to_string(),
		},
		child: KeyedNodes::with_data(vec![
			(
				key(0),
				GroupA::Widget(Box::new(
					WidgetData::<GroupAWidget>("foo bar2".to_string())
				)),
			),
		]),
	});

	let ctx = Context::new(MyDiffer);
	let path = diff::Path::new();
	let test_a = test_a.expand_widgets(None, &path);
	let test_b = test_b.expand_widgets(Some(&test_a), &path);
	test_b.diff(&path, &test_a, &ctx);
}
