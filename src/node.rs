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
    fn has(&self, event_name: &str) -> bool;
    fn send(&mut self, event_name: &str, event: EA);
}

#[derive(Debug, Clone)]
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

impl <T> Default for ParamsEventsWrapper<T> {
    #[inline]
    fn default() -> Self {
        ParamsEventsWrapper(None)
    }
}
