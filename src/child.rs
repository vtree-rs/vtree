use std::marker::PhantomData;
use ordermap::OrderMap;
use std::convert::{From, Into};
use std::ops::{Deref, DerefMut};
use key::Key;
use std::option::Option as StdOption;
use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct Single<G, AN>
    where G: Into<AN>
{
    node: Box<AN>,
    pd: PhantomData<G>,
}

impl<G, AN> Single<G, AN>
    where G: Into<AN>
{
    pub fn new(node: G) -> Single<G, AN> {
        Single {
            node: Box::new(node.into()),
            pd: PhantomData,
        }
    }
}

impl<G, AN> Deref for Single<G, AN>
    where G: Into<AN>
{
    type Target = AN;

    fn deref(&self) -> &AN {
        &self.node
    }
}

impl<G, AN> DerefMut for Single<G, AN>
    where G: Into<AN>
{
    fn deref_mut(&mut self) -> &mut AN {
        &mut self.node
    }
}

impl<G, AN> From<G> for Single<G, AN>
    where G: Into<AN>
{
    fn from(node: G) -> Single<G, AN> {
        Single::new(node)
    }
}


#[derive(Debug, Clone)]
pub struct Option<G, AN>
    where G: Into<AN>
{
    node: StdOption<Box<AN>>,
    pd: PhantomData<G>,
}

impl<G, AN> Option<G, AN>
    where G: Into<AN>
{
    pub fn new(node: StdOption<G>) -> Option<G, AN> {
        Option {
            node: node.map(|n| Box::new(n.into())),
            pd: PhantomData,
        }
    }
}

impl<G, AN> Deref for Option<G, AN>
    where G: Into<AN>
{
    type Target = StdOption<Box<AN>>;

    fn deref(&self) -> &StdOption<Box<AN>> {
        &self.node
    }
}

impl<G, AN> From<StdOption<G>> for Option<G, AN>
    where G: Into<AN>
{
    fn from(node: StdOption<G>) -> Option<G, AN> {
        Option::new(node)
    }
}

impl<G, AN> From<G> for Option<G, AN>
    where G: Into<AN>
{
    fn from(node: G) -> Option<G, AN> {
        Some(node).into()
    }
}

#[derive(Debug, Clone)]
pub struct Multi<G, AN>
    where G: Into<AN>
{
    nodes: OrderMap<Key, AN>,
    pd: PhantomData<G>,
}

impl<G, AN> Multi<G, AN>
    where G: Into<AN>
{
    pub fn new() -> Multi<G, AN> {
        Multi {
            nodes: OrderMap::new(),
            pd: PhantomData,
        }
    }

    pub fn with_capacity(cap: usize) -> Multi<G, AN> {
        Multi {
            nodes: OrderMap::with_capacity(cap),
            pd: PhantomData,
        }
    }

    pub fn get_by_key(&self, key: &Key) -> StdOption<&AN> {
        self.nodes.get(key)
    }

    pub fn push(&mut self, key: Key, node: G) {
        use ::ordermap::Entry;
        match self.nodes.entry(key) {
            Entry::Occupied(e) => panic!("multiple nodes using same key `{}`", e.key()),
            Entry::Vacant(e) => e.insert(node.into()),
        };
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a Key, &'a AN)> + 'a {
        self.nodes.iter()
    }

    pub fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = (&'a Key, &'a mut AN)> + 'a {
        self.nodes.iter_mut()
    }

    pub fn diff<'a>(&'a self,
                    last: &'a Multi<G, AN>)
                    -> impl Iterator<Item = (&'a Key, usize, StdOption<&'a AN>, StdOption<&'a AN>)> + 'a {
        last.nodes
            .iter()
            .enumerate()
            .filter_map(move |(i, (k, n))| {
                if !self.nodes.contains_key(k) {
                    // removed
                    Some((k, i, None, Some(n)))
                } else {
                    None
                }
            })
            .chain(self.nodes.iter().enumerate().map(move |(i, (k, n))| {
                // unchanged or added
                (k, i, Some(n), last.nodes.get(k))
            }))
    }

    pub fn diff_reordered<'a>(&'a self,
                    last: &'a Multi<G, AN>)
                    -> impl Iterator<Item = (usize, usize)> + 'a {
        let curr_it = self.nodes
            .keys()
            .enumerate()
            .filter(move |&(_, k)| !last.nodes.contains_key(k));

        let last_it = last.nodes
            .keys()
            .filter(move |k| self.nodes.contains_key(k))
            .enumerate();

        curr_it
            .merge_by(last_it, |a, b| a.0 <= b.0)
            .enumerate()
            .map(move |(l_index, (_, l_key))| {
                let c_index = self.nodes.get_pair_index(l_key).unwrap().0;
                (c_index, l_index)
            })
            .filter(|&(c_i, l_i)| c_i != l_i)
    }
}

pub trait IntoMultiEntry<G, AN>
    where G: Into<AN>
{
    fn into_multi_entry(self) -> (Key, G);
}

impl <G, AN> IntoMultiEntry<G, AN> for (Key, G)
    where G: Into<AN>
{
    fn into_multi_entry(self) -> (Key, G) {
        self
    }
}

impl <'a, G, AN> IntoMultiEntry<G, AN> for &'a (Key, G)
    where G: Into<AN> + Clone
{
    fn into_multi_entry(self) -> (Key, G) {
        (self.0.clone(), self.1.clone())
    }
}

impl <G, AN, IME, I> From<I> for Multi<G, AN>
    where G: Into<AN>,
          IME: IntoMultiEntry<G, AN>,
          I: IntoIterator<Item = IME>
{
    fn from(nodes: I) -> Multi<G, AN> {
        let it = nodes.into_iter();
        let mut multi = Multi::with_capacity(it.size_hint().0);
        for e in it {
            let e = e.into_multi_entry();
            multi.push(e.0, e.1);
        }
        multi
    }
}