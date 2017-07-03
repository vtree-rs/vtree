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
