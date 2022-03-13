luao3 [![Crates.io](https://img.shields.io/crates/v/luao3)](https://crates.io/crates/luao3) [![docs.rs](https://img.shields.io/docsrs/luao3)](https://docs.rs/luao3)
=====
Lua bindings for Rust, oriented around macros.

Modeled loosely after [PyO3](https://pyo3.rs/). Based on [mlua](https://docs.rs/mlua).

**WARNING**: This software is *ALPHA* quality. Expect breaking changes and API removals/additions.

## Examples
```rust
use luao3::prelude::*;
use mlua::prelude::*;

#[derive(Debug, FromLua, ToLua)]
struct Foo {
    foo: String,
    #[lua(default)]
    bar: Vec<String>
}

#[lua_function]
pub fn bar(a: Foo) -> LuaResult<Foo> {
    Ok(Foo {
        foo: format!("baz{}", a.foo),
        bar: vec!["foo".into(), "baz".into(), a.bar.get(0).cloned()
            .unwrap_or_else(|| "baz".into())]
    })
}

#[lua_function]
pub fn baz(txt: String) -> LuaResult<i32> {
    txt.parse::<i32>().map_err(mlua::Error::external)
}

luao3::declare_simple_module! {
    name => foobar,
    members => {
        fn bar,
        fn baz as baz2
    }
}
```
