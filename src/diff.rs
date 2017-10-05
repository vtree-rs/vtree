use key::Key;
use std::fmt;
use std::iter::{FromIterator, IntoIterator};
use std::marker::PhantomData;
use std::fmt::Debug;
use std::mem;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum PathIndexEntry {
    Key(Key, usize),
    /// Used for Single and Option children.
    Empty,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum PathEntry {
    Key(Key),
    /// Used for Single and Option children.
    Empty,
}

impl fmt::Display for PathEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PathEntry::Key(ref k) => write!(f, "{}", k),
            PathEntry::Empty => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Path {
    path: Vec<PathEntry>,
}

impl Path {
    pub fn new() -> Path {
        Path { path: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a PathEntry> {
        self.path.iter()
    }
}

impl FromIterator<PathEntry> for Path {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = PathEntry>,
    {
        let p: Vec<_> = iter.into_iter().collect();
        Path { path: p }
    }
}


impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, e) in self.path.iter().enumerate() {
            if 0 < i {
                write!(f, ".")?;
            }
            write!(f, "{}", e)?;
        }
        Ok(())
    }
}

pub struct SimplePathFrame<'a> {
    parent: Option<&'a SimplePathFrame<'a>>,
    path_entry: PathEntry,
}


impl<'a> SimplePathFrame<'a> {
    pub fn new() -> SimplePathFrame<'a> {
        SimplePathFrame {
            parent: None,
            path_entry: PathEntry::Empty,
        }
    }

    pub fn add_key(&'a self, key: Key) -> SimplePathFrame<'a> {
        SimplePathFrame {
            parent: Some(self),
            path_entry: PathEntry::Key(key),
        }
    }

    pub fn add_empty(&'a self) -> SimplePathFrame<'a> {
        SimplePathFrame {
            parent: Some(self),
            path_entry: PathEntry::Empty,
        }
    }

    pub fn parent(&'a self) -> Option<&'a SimplePathFrame<'a>> {
        self.parent.as_ref().map(|pf| *pf)
    }

    pub fn path_entry(&self) -> &PathEntry {
        &self.path_entry
    }

    pub fn iter(&'a self) -> SimplePathFrameIter<'a> {
        SimplePathFrameIter(Some(self))
    }

    pub fn to_path(&self) -> Path {
        self.iter().map(|spf| spf.path_entry().clone()).collect()
    }
}

pub struct SimplePathFrameIter<'a>(Option<&'a SimplePathFrame<'a>>);

impl<'a> Iterator for SimplePathFrameIter<'a> {
    type Item = &'a SimplePathFrame<'a>;

    fn next(&mut self) -> Option<&'a SimplePathFrame<'a>> {
        let next = match self.0 {
            Some(ref pf) => pf.parent(),
            None => None,
        };
        mem::replace(&mut self.0, next)
    }
}

pub struct PathFrame<'a, AN: 'a> {
    parent: Option<&'a PathFrame<'a, AN>>,
    node: &'a AN,
    path_index_entry: PathIndexEntry,
}

impl<'a, AN> PathFrame<'a, AN> {
    pub fn new(node: &'a AN) -> PathFrame<'a, AN> {
        PathFrame {
            parent: None,
            node: node,
            path_index_entry: PathIndexEntry::Empty,
        }
    }

    pub fn add_key(&'a self, key: Key, index: usize, node: &'a AN) -> PathFrame<'a, AN> {
        PathFrame {
            parent: Some(self),
            node: node,
            path_index_entry: PathIndexEntry::Key(key, index),
        }
    }

    pub fn add_empty(&'a self, node: &'a AN) -> PathFrame<'a, AN> {
        PathFrame {
            parent: Some(self),
            node: node,
            path_index_entry: PathIndexEntry::Empty,
        }
    }

    pub fn parent(&'a self) -> Option<&'a PathFrame<'a, AN>> {
        self.parent.as_ref().map(|pf| *pf)
    }

    pub fn node(&self) -> &AN {
        &self.node
    }

    pub fn path_index_entry(&self) -> &PathIndexEntry {
        &self.path_index_entry
    }

    pub fn to_path_entry(&self) -> PathEntry {
        match self.path_index_entry {
            PathIndexEntry::Key(ref key, _) => PathEntry::Key(key.clone()),
            PathIndexEntry::Empty => PathEntry::Empty,
        }
    }

    pub fn iter(&'a self) -> PathFrameIter<'a, AN> {
        PathFrameIter(Some(self))
    }

    pub fn to_path(&self) -> Path {
        self.iter().map(|pf| pf.to_path_entry()).collect()
    }
}

pub struct PathFrameIter<'a, AN: 'a>(Option<&'a PathFrame<'a, AN>>);

impl<'a, AN> Iterator for PathFrameIter<'a, AN> {
    type Item = &'a PathFrame<'a, AN>;

    fn next(&mut self) -> Option<&'a PathFrame<'a, AN>> {
        let next = match self.0 {
            Some(ref pf) => pf.parent(),
            None => None,
        };
        mem::replace(&mut self.0, next)
    }
}

pub trait Differ<CTX, AN>: Debug {
    fn diff_added(&mut self, ctx: &mut Context<CTX, AN>, curr: &PathFrame<AN>);

    fn diff_removed(&mut self, ctx: &mut Context<CTX, AN>, last: &PathFrame<AN>);

    fn diff_replaced(
        &mut self,
        ctx: &mut Context<CTX, AN>,
        curr: &PathFrame<AN>,
        last: &PathFrame<AN>,
    ) {
        self.diff_removed(ctx, curr);
        self.diff_added(ctx, last);
    }

    fn diff_params_changed(
        &mut self,
        ctx: &mut Context<CTX, AN>,
        curr: &PathFrame<AN>,
        last: &PathFrame<AN>,
    );

    fn diff_reordered<I: Iterator<Item = (usize, usize)>>(
        &mut self,
        ctx: &mut Context<CTX, AN>,
        parent: &PathFrame<AN>,
        indices: I,
    );

    #[inline]
    fn on_enter_curr(&mut self, _ctx: &mut Context<CTX, AN>, _curr: &PathFrame<AN>) {}

    #[inline]
    fn on_exit_curr(&mut self, _ctx: &mut Context<CTX, AN>, _curr: &PathFrame<AN>) {}

    #[inline]
    fn on_enter_last(&mut self, _ctx: &mut Context<CTX, AN>, _last: &PathFrame<AN>) {}

    #[inline]
    fn on_exit_last(&mut self, _ctx: &mut Context<CTX, AN>, _last: &PathFrame<AN>) {}
}

#[derive(Debug)]
pub struct Context<CTX, AN> {
    // pub widgets: HashMap<diff::Path, Box<WidgetDataTrait<G>>>,
    pub ctx: CTX,
    pd: PhantomData<AN>,
}

impl<CTX, AN> Context<CTX, AN> {
    pub fn new(ctx: CTX) -> Context<CTX, AN> {
        Context {
            ctx: ctx,
            pd: PhantomData,
        }
    }
}
