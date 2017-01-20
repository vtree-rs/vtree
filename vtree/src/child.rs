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


pub enum MultiDiff<'a, AN: 'a> {
    Node(&'a Key, usize, StdOption<&'a AN>, StdOption<&'a AN>),
    Reordered(Vec<(usize, usize)>),
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

    pub fn with_data<T>(nodes: T) -> Multi<G, AN>
        where T: IntoIterator<Item = (Key, G)>
    {
        let it = nodes.into_iter();
        let mut nodes = Multi::with_capacity(it.size_hint().0);
        for n in it {
            nodes.push(n.0, n.1);
        }
        nodes
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
                    -> impl Iterator<Item = MultiDiff<'a, AN>> + 'a {
        last.ordered
            .iter()
            .enumerate()
            .filter_map(move |(i, k)| {
                if !self.nodes.contains_key(k) {
                    // removed
                    Some(MultiDiff::Node(k, i, None, Some(last.nodes.get(k).unwrap())))
                } else {
                    None
                }
            })
            .chain(self.ordered.iter().enumerate().map(move |(i, k)| {
                let n_cur = self.nodes.get(k).unwrap();
                // unchanged or added
                MultiDiff::Node(k, i, Some(n_cur), last.nodes.get(k))
            }))
            .chain(self.ordered.iter().enumerate().filter_map(move |(i, k)| {
                if let Some(i_last) = last.ordered.iter().position(|k_last| k == k_last) {
                    if i != i_last {
                        Some(MultiDiff::Reordered(vec![(i, i_last)]))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }))
    }
}
