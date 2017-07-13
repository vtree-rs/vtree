pub trait Pipeline<V> {
    type Item;

    fn send(&mut self, value: V) -> Option<Self::Item>;

    fn map<B, F>(self, f: F) -> Map<Self, F>
        where F: FnMut(Self::Item) -> B
    {
        Map {next: self, f: f}
    }
}

pub struct Start;

impl<V> Pipeline for Start
    where F: FnMut(I::Item) -> I::Item
{
    type Item = V;

    #[inline]
    fn send(&mut self, value: V) -> Option<Self::Item> {
        value
    }
}

#[must_use = "event pipeline adaptors are lazy and do nothing unless consumed"]
pub struct Map<N, F> {
    next: N,
    f: F,
}

impl<B, V, N, F> Pipeline for Map<N, F>
    where F: FnMut(I::Item) -> B
          N: Pipeline<Item=B>
{
    type Item = B;

    #[inline]
    fn send(&mut self, value: V) -> Self::Item {
        self.f(self.next.send(value))
    }
}


#[must_use = "event pipeline adaptors are lazy and do nothing unless consumed"]
pub struct Filter<N, F> {
    next: N,
    f: F,
}

impl<B, V, N, F> Pipeline for Filter<N, F>
    where F: FnMut(&I::Item) -> bool
          N: Pipeline<Item=B>
{
    type Item = V;

    #[inline]
    fn send(&mut self, value: Self::Item) -> Self::Item {
        self.f(self.next.send(value))
    }
}
