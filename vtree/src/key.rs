use std::fmt::{self, Write};
use std::iter::IntoIterator;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Key {
	U64(u64),
	String(String),
	Str(&'static str),
	Bytes(Vec<u8>),
}

impl fmt::Display for Key {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			&Key::U64(ref n) => write!(f, "{}", n),
			&Key::String(ref s) => write!(f, "{}", s),
			&Key::Str(s) => write!(f, "{}", s),
			&Key::Bytes(ref bytes) => {
				let mut s = String::with_capacity(bytes.len() * 2 + 2);
				try!(write!(&mut s, "0x"));
				for &b in bytes {
					try!(write!(&mut s, "{:x}", b));
				}
				write!(f, "{}", s)
			},
		}
	}
}

pub enum KeyedDiff<'a, G: 'a> {
	Added(&'a Key, usize, &'a G),
	Removed(&'a Key, usize, &'a G),
	Unchanged(&'a Key, usize, &'a G, &'a G),
	Reordered(usize, usize),
}

#[derive(Debug, Clone)]
pub struct KeyedNodes<G> {
	ordered: Vec<Key>,
	nodes: HashMap<Key, G>,
}

impl<G> KeyedNodes<G> {
	pub fn new() -> KeyedNodes<G> {
		KeyedNodes {
			ordered: Vec::new(),
			nodes: HashMap::new(),
		}
	}

	pub fn with_capacity(cap: usize) -> KeyedNodes<G> {
		KeyedNodes {
			ordered: Vec::with_capacity(cap),
			nodes: HashMap::with_capacity(cap),
		}
	}

	pub fn with_data<T>(nodes: T) -> KeyedNodes<G>
		where T: IntoIterator<Item=(Key, G)>
	{
		let it = nodes.into_iter();
		let mut nodes = KeyedNodes::with_capacity(it.size_hint().0);
		for n in it {
			nodes.push(n.0, n.1);
		}
		nodes
	}

	pub fn get_by_key(&self, key: &Key) -> Option<&G> {
		self.nodes.get(key)
	}

	pub fn get_by_index(&self, index: usize) -> Option<&G> {
		let key = self.ordered.get(index);
		if let Some(ref key) = key {
			self.nodes.get(key)
		} else {
			None
		}
	}

	pub fn push(&mut self, key: Key, node: G) {
		if self.nodes.insert(key.clone(), node).is_some() {
			panic!("multiple nodes using same key \"{}\"", key);
		}
		self.ordered.push(key);
	}

	pub fn inplace_map<F>(&mut self, func: F)
		where F: Fn(&Key, G) -> G,
	{
		let keys: Vec<_> = self.nodes.keys().cloned().collect();
		for key in keys {
			let node = self.nodes.remove(&key).unwrap();
			let node = func(&key, node);
			self.nodes.insert(key, node);
		}
	}

	pub fn iter_ordered<'a>(&'a self) -> impl Iterator<Item=(&'a Key, &'a G)> + 'a {
		self.ordered.iter().map(move |key| {
			(key, self.nodes.get(key).unwrap())
		})
	}

	pub fn diff<'a>(&'a self, last: &'a KeyedNodes<G>) -> impl Iterator<Item=KeyedDiff<'a, G>> + 'a {
	last.ordered
		.iter()
		.enumerate()
		.filter_map(move |(i, k)| {
			if !self.nodes.contains_key(k) {
				Some(KeyedDiff::Removed(k, i, last.nodes.get(k).unwrap()))
			} else {
				None
			}
		})
		.chain(
			self.ordered.iter().enumerate().map(move |(i, k)| {
				let n_cur = self.nodes.get(k).unwrap();
				if let Some(ref n_last) = last.nodes.get(k) {
					KeyedDiff::Unchanged(k, i, n_cur, n_last)
				} else {
					KeyedDiff::Added(k, i, n_cur)
				}
			})
		)
		.chain(
			self.ordered.iter().enumerate().filter_map(move |(i, k)| {
				if let Some(i_last) = last.ordered.iter().position(|k_last| k == k_last) {
					if i != i_last {
						Some(KeyedDiff::Reordered(i, i_last))
					} else {
						None
					}
				} else {
					None
				}
			})
		)
	}
}
