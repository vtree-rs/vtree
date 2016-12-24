use key::Key;
use std::fmt;
use std::iter::{IntoIterator, FromIterator, Extend};
use std::vec::IntoIter;

#[derive(Clone, Debug)]
pub enum PathNode {
    Key(Key),
    Field(&'static str),
}

#[derive(Clone, Debug)]
pub struct Path {
    path: Vec<PathNode>,
}

impl Path {
    pub fn new() -> Path {
        Path { path: Vec::new() }
    }

    pub fn add_key(&self, k: Key) -> Path {
        let mut p = self.path.clone();
        p.push(PathNode::Key(k));
        Path { path: p }
    }

    pub fn add_field(&self, n: &'static str) -> Path {
        let mut p = self.path.clone();
        p.push(PathNode::Field(n));
        Path { path: p }
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

    pub fn split_at(&self, mid: usize) -> (Path, Path) {
        let (left, right) = self.path.split_at(mid);
        (Path { path: left.to_vec() }, Path { path: right.to_vec() })
    }

    fn extend<T>(&self, iter: T) -> Path
        where T: IntoIterator<Item = PathNode>
    {
        let mut p = self.path.clone();
        p.extend(iter);
        Path { path: p }
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a PathNode> {
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

impl FromIterator<PathNode> for Path {
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item = PathNode>
    {
        let p: Vec<_> = iter.into_iter().collect();
        Path { path: p }
    }
}


impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self.path.iter().fold(String::new(), |acc, p| {
            acc +
            &match p {
                &PathNode::Key(ref k) => format!(".{}", k),
                &PathNode::Field(ref n) => format!("::{}", n),
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

#[derive(Debug)]
pub struct Context<D> {
    // pub widgets: HashMap<diff::Path, Box<WidgetDataTrait<G>>>,
    pub differ: D,
}

impl<D> Context<D> {
    pub fn new(differ: D) -> Context<D> {
        Context { differ: differ }
    }
}
