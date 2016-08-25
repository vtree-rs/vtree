#![feature(plugin)]
#![plugin(vtree_macros)]

extern crate vtree;

use vtree::key::{Key, KeyedDiff, KeyedNodes};
use vtree::widget::{Widget, WidgetData};
use vtree::diff::{self, Diff};

#[derive(Debug, Clone, PartialEq)]
struct AParams {
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

pub trait Differ {
	fn diff_group_a<'a>(
		&self,
		path: &diff::Path,
		curr: &GroupA,
		diff: diff::Diff,
	);

	fn reorder_a_child(
		&self,
		path: &diff::Path,
		index_curr: usize,
		index_last: usize,
	);

	fn params_changed_a(&self, path: &diff::Path, curr: &A, last: &A);
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

impl GroupA {
	pub fn expand_widgets(self, last: Option<&GroupA>, path: &diff::Path) -> GroupA {
		let mut curr = if let GroupA::Widget(widget_data) = self {
			match widget_data.render() {
				Some(result) => result,
				None => {
					let last = last.unwrap();
					if let &GroupA::Widget(..) = last {
						panic!("Widgets not allowed in last in `{}`", path);
					}
					return last.clone();
				}
			}
		} else {
			self
		};

		match curr {
			GroupA::A(ref mut curr_node) => {
				if let Some(&GroupA::A(ref last_node)) = last {
					let path_field = path.add_node_field("child");
					curr_node.child.inplace_map(|key, node| {
						node.expand_widgets(last_node.child.get_by_key(key), &path_field.add_key(key.clone()))
					});
				} else {
					let path_field = path.add_node_field("child");
					curr_node.child.inplace_map(|key, node| {
						node.expand_widgets(None, &path_field.add_key(key.clone()))
					});
				}
			},
			GroupA::Widget(_) => unreachable!(),
		}

		curr
	}

	pub fn diff<'a, D: Differ>(
		&self,
		path: &diff::Path,
		last: &GroupA,
		ctx: &Context<D>,
	) {
		match self {
			&GroupA::A(ref curr_node) => {
				if let &GroupA::A(ref last_node) = last {
					if curr_node.params != last_node.params {
						ctx.differ.params_changed_a(path, curr_node, &last_node);
					}
					let curr_path = path.add_node_field("child");
					for diff in curr_node.child.diff(&last_node.child) {
						match diff {
							KeyedDiff::Added(key, index, node) => {
								ctx.differ.diff_group_a(&curr_path.add_key(key.clone()), &node, Diff::Added);
							},
							KeyedDiff::Removed(key, index, node) => {
								ctx.differ.diff_group_a(&curr_path.add_key(key.clone()), &node, Diff::Removed);
							},
							KeyedDiff::Unchanged(key, index, curr_child, last_child) => {
								curr_child.diff(&curr_path.add_key(key.clone()), last_child, ctx);
							},
							KeyedDiff::Reordered(i_cur, i_last) => {
								ctx.differ.reorder_a_child(path, i_cur, i_last);
							},
						}
					}
				} else {
					// TODO: call node removed hook
					ctx.differ.diff_group_a(path, &self, Diff::Replaced);
				}
			},
			&GroupA::Widget(_) => unreachable!(),
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
				Key::U64(0),
				GroupA::Widget(Box::new(
					WidgetData::<GroupAWidget>("foo bar".to_string())
				)),
			),

			(
				Key::U64(1),
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
				Key::U64(0),
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
