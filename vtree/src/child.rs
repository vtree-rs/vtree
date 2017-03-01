use std::marker::PhantomData;
use std::collections::HashMap;
use std::convert::{From, Into};
use std::ops::Deref;
use key::Key;
use std::option::Option as StdOption;

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

impl<G, AN> From<G> for Single<G, AN>
    where G: Into<AN>
{
    fn from(node: G) -> Single<G, AN> {
        Single::new(node)
    }
}


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
    ordered: Vec<Key>,
    nodes: HashMap<Key, AN>,
    pd: PhantomData<G>,
}

impl<G, AN> Multi<G, AN>
    where G: Into<AN>
{
    pub fn new() -> Multi<G, AN> {
        Multi {
            ordered: Vec::new(),
            nodes: HashMap::new(),
            pd: PhantomData,
        }
    }

    pub fn with_capacity(cap: usize) -> Multi<G, AN> {
        Multi {
            ordered: Vec::with_capacity(cap),
            nodes: HashMap::with_capacity(cap),
            pd: PhantomData,
        }
    }

    pub fn get_by_key(&self, key: &Key) -> StdOption<&AN> {
        self.nodes.get(key)
    }

    pub fn get_by_index(&self, index: usize) -> StdOption<&AN> {
        let key = self.ordered.get(index);
        if let Some(ref key) = key {
            self.nodes.get(key)
        } else {
            None
        }
    }

    pub fn push(&mut self, key: Key, node: G) {
        if self.nodes.insert(key.clone(), node.into()).is_some() {
            panic!("multiple nodes using same key \"{}\"", key);
        }
        self.ordered.push(key);
    }

    pub fn inplace_map<F>(&mut self, func: F)
        where F: Fn(&Key, AN) -> AN
    {
        for key in self.ordered.iter() {
            let node = self.nodes.remove(key).unwrap();
            let node = func(key, node);
            self.nodes.insert(key.clone(), node);
        }
    }

    pub fn iter_ordered<'a>(&'a self) -> impl Iterator<Item = (&'a Key, &'a AN)> + 'a {
        self.ordered.iter().map(move |key| (key, self.nodes.get(key).unwrap()))
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a Key, &'a AN)> + 'a {
        self.nodes.iter()
    }

    pub fn diff<'a>(&'a self,
                    last: &'a Multi<G, AN>)
                    -> impl Iterator<Item = (&'a Key, usize, StdOption<&'a AN>, StdOption<&'a AN>)> + 'a {
        last.ordered
            .iter()
            .enumerate()
            .filter_map(move |(i, k)| {
                if !self.nodes.contains_key(k) {
                    // removed
                    Some((k, i, None, Some(last.nodes.get(k).unwrap())))
                } else {
                    None
                }
            })
            .chain(self.ordered.iter().enumerate().map(move |(i, k)| {
                let n_cur = self.nodes.get(k).unwrap();
                // unchanged or added
                (k, i, Some(n_cur), last.nodes.get(k))
            }))
    }

    pub fn diff_reordered<'a>(&'a self,
                    last: &'a Multi<G, AN>)
                    -> impl Iterator<Item = (usize, usize)> + 'a {
        // TODO: + index to self.nodes
        let index_lookup: HashMap<_, _> = self.ordered
            .iter()
            .enumerate()
            .filter(|&(_, key)| last.nodes.contains_key(key))
            .map(|(index, key)| (key, index))
            .collect();
        let curr_it = self.ordered
            .iter()
            .filter(move |key| last.nodes.contains_key(key));
        let last_it = last.ordered
            .iter()
            .enumerate()
            .filter(move |&(_, key)| self.nodes.contains_key(key));
        curr_it
            .zip(last_it)
            .filter(|(c_key, (_, l_key))| c_key != l_key)
            .map(move |(_, (l_index, l_key))| {
                (*index_lookup.get(l_key).unwrap(), l_index)
            })
    }
}

pub trait IntoMultiEntry<G, AN>
    where G: Into<AN>
{
    fn into_multi_entry(self) -> (Key, G);
}

impl <K, G, AN> IntoMultiEntry<G, AN> for (K, G)
    where K: Into<Key>,
          G: Into<AN>
{
    fn into_multi_entry(self) -> (Key, G) {
        (self.0.into(), self.1)
    }
}

impl <'a, K, G, AN> IntoMultiEntry<G, AN> for &'a (K, G)
    where K: 'a + Clone + Into<Key>,
          G: 'a + Clone + Into<AN>
{
    fn into_multi_entry(self) -> (Key, G) {
        (self.0.clone().into(), self.1.clone())
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
