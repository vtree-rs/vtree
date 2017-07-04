#![feature(plugin)]
#![feature(proc_macro)]

extern crate vtree_markup;

use vtree_markup::markup;

fn main() {
    let a = 42;
    markup! {
        Container {
            Button@"1. button" test="foo" bool_true? Label "foo bar"
            Button@"2. button"
                test="foo"
                dynamic=(match a {
                    1 => "foo",
                    2 => "bar",
                })
                Label {"foo bar: `"@1 (a)@2 "`"@3}
            Button@"3. button" test="foo" Label {"foo bar: `" (a) "`"}
        }
    }
}
