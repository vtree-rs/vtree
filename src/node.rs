use std::fmt::Debug;
use std::any::TypeId;
use std::collections::HashMap;

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

pub trait TypeMapEntry: Debug + 'static {
    fn boxed_clone(&self) -> Box<TypeMapEntry>;
}

impl<T: Clone + Debug + 'static> TypeMapEntry for T {
    fn boxed_clone(&self) -> Box<TypeMapEntry> {
        Box::new(self.clone())
    }
}

impl Clone for Box<TypeMapEntry> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

#[derive(Clone, Debug)]
pub struct TypeMap {
    map: HashMap<TypeId, Box<TypeMapEntry>>,
}

impl TypeMap {
    pub fn new() -> TypeMap {
        TypeMap {
            map: HashMap::new(),
        }
    }

    pub fn insert<T: TypeMapEntry>(&mut self, v: T) -> Option<T> {
        self.map
            .insert(TypeId::of::<T>(), Box::new(v) as Box<TypeMapEntry>)
            .map(|v| unsafe {
                *Box::from_raw(Box::into_raw(v) as *mut T)
            })
    }

    pub fn contains<T: TypeMapEntry>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    pub fn get<T: TypeMapEntry>(&self) -> Option<&T> {
        self.map.get(&TypeId::of::<T>()).map(|v| unsafe {
            &*(v.as_ref() as *const TypeMapEntry as *const T)
        })
    }

    pub fn get_mut<T: TypeMapEntry>(&mut self) -> Option<&mut T> {
        self.map.get_mut(&TypeId::of::<T>()).map(|v| unsafe {
            &mut *(v.as_mut() as *mut TypeMapEntry as *mut T)
        })
    }

    pub fn remove<T: TypeMapEntry>(&mut self) -> Option<T> {
        self.map
            .remove(&TypeId::of::<T>())
            .map(|v| unsafe {
                *Box::from_raw(Box::into_raw(v) as *mut T)
            })
    }
}
