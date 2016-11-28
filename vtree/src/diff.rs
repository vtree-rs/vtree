use key::Key;
use std::fmt;
use std::iter::IntoIterator;
use std::vec::IntoIter;

#[derive(Clone, Debug)]
pub enum PathNode {
	Key(Key),
	NodeField(&'static str),
}

#[derive(Clone, Debug)]
pub struct Path {
	path: Vec<PathNode>,
}

impl Path {
	pub fn new() -> Path {
		Path {
			path: Vec::new(),
		}
	}

	pub fn add_key(&self, k: Key) -> Path {
		let mut p = self.path.clone();
		p.push(PathNode::Key(k));
		Path {
			path: p,
		}
	}

	pub fn add_node_field(&self, n: &'static str) -> Path {
		let mut p = self.path.clone();
		p.push(PathNode::NodeField(n));
		Path {
			path: p,
		}
	}

	pub fn iter<'a>(&'a self) -> impl Iterator<Item=&'a PathNode> {
		self.path.iter()
	}
}

impl IntoIterator for Path {
	type Item = PathNode;
	type IntoIter = IntoIter<PathNode>;

	fn into_iter(self) -> IntoIter<PathNode> {
		self.path.into_iter()
	}
}

impl fmt::Display for Path {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let s = self.path.iter().fold(String::new(), |acc, p| {
			acc + &match p {
				&PathNode::Key(ref k) => format!(".{}", k),
				&PathNode::NodeField(ref n) => format!("::{}", n),
			}
		});

		write!(f, "{}", s)
	}
}

#[derive(Debug)]
pub enum Diff {
	Replaced,
	Added,
	Removed,
}
