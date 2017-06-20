#![feature(plugin)]
#![feature(proc_macro)]

extern crate vtree_markup;

use vtree_markup::markup;

fn main() {
    markup! {
        container {
            button@"1. button" test="foo" bool_true? label "foo bar"
            button@"2. button"
                test="foo"
                dynamic=(match a {
                    1 => "foo",
                    2 => "bar",
                })
                label {"foo bar: `"@1 (a)@2 "`"@3}
            button@"3. button" test="foo" label {"foo bar: `" (a) "`"}
        }
    }
}
