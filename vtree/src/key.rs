use std::fmt::{self, Write};
use std::rc::Rc;
use std::convert::{From, Into};
use std::borrow::Borrow;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Key {
    U64(u64),
    I64(i64),
    String(Rc<String>),
    Str(&'static str),
    Bytes(Rc<Vec<u8>>),
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

pub fn key<T: Into<Key>>(key: T) -> Key {
    key.into()
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Key::U64(ref n) => write!(f, "{}", n),
            &Key::I64(ref n) => write!(f, "{}", n),
            &Key::String(ref s) => write!(f, "{}", s),
            &Key::Str(s) => write!(f, "{}", s),
            &Key::Bytes(ref bytes) => {
                let mut s = String::with_capacity(bytes.len() * 2 + 2);
                try!(write!(&mut s, "0x"));
                for &b in bytes.iter() {
                    try!(write!(&mut s, "{:x}", b));
                }
                write!(f, "{}", s)
            }
        }
    }
}
