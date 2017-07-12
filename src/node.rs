use std::fmt::Debug;

pub struct BuilderParams;
pub struct BuilderChild;

pub trait BuilderSetter<K, V> {
    fn builder_set(&mut self, value: V);
}

pub trait Params<PB>
    where PB: BuilderSetter<BuilderParams, Self>,
          Self: Sized,
{
    type Builder;

    fn builder(parent_builder: PB) -> Self::Builder;
}

pub trait ParamsEvents<EA>: Debug {
    fn send(&mut self, event_name: &'static str, event: EA);
}

#[derive(Debug)]
pub struct ParamsEventsWrapper<T>(pub Option<T>);

/// Does a *fake* eq check by returning true in all cases and making it a neutral element in
/// parent eq impl.
impl <T> PartialEq for ParamsEventsWrapper<T> {
    #[inline]
    fn eq(&self, _other: &ParamsEventsWrapper<T>) -> bool {
        true
    }
}

impl <T> Eq for ParamsEventsWrapper<T> {}

/// Does a *fake* clone by actually returning a new empty ParamsEventsWrapper instance.
impl <T> Clone for ParamsEventsWrapper<T> {
    #[inline]
    fn clone(&self) -> Self {
        ParamsEventsWrapper(None)
    }
}

impl <T> Default for ParamsEventsWrapper<T> {
    #[inline]
    fn default() -> Self {
        ParamsEventsWrapper(None)
    }
}
