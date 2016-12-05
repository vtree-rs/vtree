use std::fmt::{self, Write};
use std::iter::IntoIterator;
use std::collections::HashMap;
use std::rc::Rc;
use std::convert::{From, Into};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Key {
	U64(u64),
	I64(i64),
	String(Rc<String>),
	Str(&'static str),
	Bytes(Rc<Vec<u8>>),
}

macro_rules! impl_from_int_for_key {
	($tyu:ty, $tyi:ty) => {
		impl From<$tyu> for Key {
			fn from(v: $tyu) -> Key {
				Key::U64(v as u64)
			}
		}

		impl From<$tyi> for Key {
			fn from(v: $tyi) -> Key {
				Key::I64(v as i64)
			}
		}
	};
}

impl_from_int_for_key!(u8, i8);
impl_from_int_for_key!(u16, i16);
impl_from_int_for_key!(u32, i32);
impl_from_int_for_key!(u64, i64);
impl_from_int_for_key!(usize, isize);

impl From<String> for Key {
	fn from(v: String) -> Key {
		Key::String(Rc::new(v))
	}
}

impl From<&'static str> for Key {
	fn from(v: &'static str) -> Key {
		Key::Str(v)
	}
}

impl From<Vec<u8>> for Key {
	fn from(v: Vec<u8>) -> Key {
		Key::Bytes(Rc::new(v))
	}
}

pub fn key<T: Into<Key>>(key: T) -> Key {
	key.into()
}

impl fmt::Display for Key {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			&Key::U64(ref n) => write!(f, "{}", n),
			&Key::I64(ref n) => write!(f, "{}", n),
			&Key::String(ref s) => write!(f, "{}", s),
			&Key::Str(s) => write!(f, "{}", s),
			&Key::Bytes(ref bytes) => {
				let mut s = String::with_capacity(bytes.len() * 2 + 2);
				try!(write!(&mut s, "0x"));
				for &b in bytes.iter() {
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
		for key in self.ordered.iter() {
			let node = self.nodes.remove(key).unwrap();
			let node = func(key, node);
			self.nodes.insert(key.clone(), node);
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
