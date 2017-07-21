use key::Key;
use std::fmt;
use std::iter::{IntoIterator, FromIterator, Extend};
use std::vec::IntoIter;
use std::marker::PhantomData;
use std::fmt::Debug;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Path {
    path: Vec<Key>,
}

impl Path {
    pub fn new() -> Path {
        Path { path: Vec::new() }
    }

    pub fn add(&self, k: Key) -> Path {
        let mut p = self.path.clone();
        p.push(k);
        Path { path: p }
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

    pub fn split_at(&self, mid: usize) -> (Path, Path) {
        let (left, right) = self.path.split_at(mid);
        (Path { path: left.to_vec() }, Path { path: right.to_vec() })
    }

    pub fn extend<T>(&self, iter: T) -> Path
        where T: IntoIterator<Item = Key>
    {
        let mut p = self.path.clone();
        p.extend(iter);
        Path { path: p }
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Key> {
        self.path.iter()
    }
}

impl IntoIterator for Path {
    type Item = Key;
    type IntoIter = IntoIter<Key>;

    fn into_iter(self) -> IntoIter<Key> {
        self.path.into_iter()
    }
}

impl FromIterator<Key> for Path {
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item = Key>
    {
        let p: Vec<_> = iter.into_iter().collect();
        Path { path: p }
    }
}


impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for key in self.path.iter() {
            write!(f, ".{}", key)?;
        }
        Ok(())
    }
}

pub trait Differ<CTX, AN>: Debug {
    fn diff_added(&mut self, ctx: &mut Context<CTX, AN>, path: &Path, index: usize, curr: &AN);
    fn diff_removed(&mut self, ctx: &mut Context<CTX, AN>, path: &Path, index: usize, last: &AN);
    fn diff_replaced(&mut self, ctx: &mut Context<CTX, AN>, path: &Path, index: usize, curr: &AN, last: &AN);
    fn diff_params_changed(&mut self, ctx: &mut Context<CTX, AN>, path: &Path, curr: &AN, last: &AN);
    fn diff_reordered<I: Iterator<Item=(usize, usize)>>(&mut self, ctx: &mut Context<CTX, AN>, path: &Path, indices: I);
}

#[derive(Debug)]
pub struct Context<CTX, AN> {
    // pub widgets: HashMap<diff::Path, Box<WidgetDataTrait<G>>>,
    pub ctx: CTX,
    pd: PhantomData<AN>,
}

impl<CTX, AN> Context<CTX, AN> {
    pub fn new(ctx: CTX) -> Context<CTX, AN> {
        Context { ctx: ctx, pd: PhantomData }
    }
}
