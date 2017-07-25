use std::fmt;
use std::rc::Rc;
use std::convert::{From, Into};
use std::borrow::Borrow;

#[derive(Debug, Eq, Hash, Clone)]
pub enum Key {
    U64(u64),
    I64(i64),
    String(Rc<String>),
    Str(&'static str),
    Bytes(Rc<Vec<u8>>),
}

impl PartialEq for Key {
    fn eq(&self, other: &Key) -> bool {
        match (self, other) {
            (&Key::U64(ref a), &Key::U64(ref b)) => a == b,
            (&Key::I64(ref a), &Key::I64(ref b)) => a == b,
            (&Key::String(ref a), &Key::String(ref b)) => a == b,
            (&Key::String(ref a), &Key::Str(b)) => a.as_str() == b,
            (&Key::Str(a), &Key::String(ref b)) => a == b.as_str(),
            (&Key::Str(a), &Key::Str(b)) => a == b,
            (&Key::Bytes(ref a), &Key::Bytes(ref b)) => a == b,
            _ => false,
        }
    }
}

macro_rules! impl_from_int_for_key {
    ($tyu:ty, $tyi:ty) => {
        impl From<$tyu> for Key {
            fn from(v: $tyu) -> Key {
                Key::U64(v as u64)
            }
        }

        impl From<$tyi> for Key {
            fn from(v: $tyi) -> Key {
                Key::I64(v as i64)
            }
        }
    };
}

impl_from_int_for_key!(u8, i8);
impl_from_int_for_key!(u16, i16);
impl_from_int_for_key!(u32, i32);
impl_from_int_for_key!(u64, i64);
impl_from_int_for_key!(usize, isize);

impl From<String> for Key {
    fn from(v: String) -> Key {
        Key::String(Rc::new(v))
    }
}

impl From<Rc<String>> for Key {
    fn from(v: Rc<String>) -> Key {
        Key::String(v)
    }
}

impl From<&'static str> for Key {
    fn from(v: &'static str) -> Key {
        Key::Str(v)
    }
}

impl From<Vec<u8>> for Key {
    fn from(v: Vec<u8>) -> Key {
        Key::Bytes(Rc::new(v))
    }
}

impl From<Rc<Vec<u8>>> for Key {
    fn from(v: Rc<Vec<u8>>) -> Key {
        Key::Bytes(v)
    }
}

impl<'a, T, O> From<&'a T> for Key
    where T: ToOwned<Owned=O> + ?Sized,
          O: Borrow<T> + Into<Key>
{
    default fn from(v: &'a T) -> Key {
        v.to_owned().into()
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Key::U64(ref n) => write!(f, "u{}", n),
            Key::I64(ref n) => write!(f, "i{}", n),
            Key::String(ref s) => write!(f, "s{}", s),
            Key::Str(s) => write!(f, "s{}", s),
            Key::Bytes(ref bytes) => {
                write!(f, "0x")?;
                for b in bytes.iter() {
                    write!(f, "{:02x}", b)?;
                }
                Ok(())
            }
        }
    }
}
