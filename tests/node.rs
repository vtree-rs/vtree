extern crate vtree;

use vtree::node::TypeMap;

#[test]
fn type_map() {
    let mut tm = TypeMap::new();

    assert_eq!(tm.insert("foo"), None);
    assert_eq!(tm.insert("bar"), Some("foo"));

    assert_eq!(tm.insert("foo".to_string()), None);
    assert_eq!(tm.insert("bar".to_string()), Some("foo".to_string()));

    assert!(tm.contains::<&str>());
    assert!(tm.contains::<String>());
    assert!(!tm.contains::<u8>());

    assert_eq!(tm.get::<&str>(), Some(&"bar"));
    assert_eq!(tm.get::<String>(), Some(&"bar".to_string()));
    assert_eq!(tm.get::<u8>(), None);

    *tm.get_mut::<&str>().unwrap() = "asd";
    assert_eq!(tm.get::<&str>(), Some(&"asd"));
    *tm.get_mut::<String>().unwrap() = "asd".to_string();
    assert_eq!(tm.get::<String>(), Some(&"asd".to_string()));

    assert_eq!(tm.remove::<&str>(), Some("asd"));
    assert!(!tm.contains::<&str>());
    assert_eq!(tm.remove::<String>(), Some("asd".to_string()));
    assert!(!tm.contains::<String>());
}
