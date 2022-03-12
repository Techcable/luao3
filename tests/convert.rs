use luao3::{tostring::LuaToString, FromLua, ToLua};
use mlua::chunk;

#[derive(Debug, FromLua, ToLua, PartialEq, Eq)]
enum CEnum {
    Foo,
    Bar,
    Baz,
    UrMum,
}

/// Tests C style enums, which convert to/from strings
#[test]
pub fn cenum() {
    use mlua::{FromLua, Lua, ToLua};
    let lua = Lua::new();
    let expected = vec![
        (CEnum::Foo, "Foo"),
        (CEnum::Bar, "Bar"),
        (CEnum::Baz, "Baz"),
        (CEnum::UrMum, "UrMum"),
    ];
    for (var, name) in expected {
        assert_eq!(
            CEnum::from_lua(name.to_lua(&lua).unwrap(), &lua).unwrap(),
            var
        );
        assert!(var
            .to_lua(&lua)
            .unwrap()
            .equals(name.to_lua(&lua).unwrap())
            .unwrap());
    }
}

#[derive(Debug, FromLua, ToLua, PartialEq, Clone)]
pub enum MixedEnum {
    Foo,
    Bar,
    Baz { k: u32 },
    Bacon(String, f64),
}

#[test]
pub fn mixed_enum() {
    use mlua::{FromLua, Lua, ToLua};
    let lua = Lua::new();
    let expected = vec![
        (MixedEnum::Foo, "Foo".to_lua(&lua).unwrap()),
        (MixedEnum::Bar, "Bar".to_lua(&lua).unwrap()),
        (
            MixedEnum::Baz { k: 52 },
            lua.load(chunk! {
                return {
                    Baz = {
                        k = 52
                    }
                }
            })
            .eval()
            .unwrap(),
        ),
        (
            MixedEnum::Bacon("cook".into(), 2.718),
            lua.load(chunk! {
                return {
                    Bacon = {"cook", 2.718}
                }
            })
            .eval()
            .unwrap(),
        ),
    ];
    for (var, value) in expected {
        assert_eq!(MixedEnum::from_lua(value.clone(), &lua).unwrap(), var);
        let tolua = var.to_lua(&lua).unwrap();
        assert_eq!(
            value.to_lua_string(&lua).unwrap(),
            tolua.to_lua_string(&lua).unwrap()
        );
    }
}

#[derive(Debug, FromLua, ToLua, Eq, PartialEq)]
pub struct SimpleStruct {
    pub a: u32,
    pub b: u32,
    c: String,
}

#[test]
fn simple_struct() {
    use mlua::{FromLua, Lua, ToLua};
    let lua = Lua::new();
    let expected: Vec<(SimpleStruct, mlua::Value)> = vec![(
        SimpleStruct {
            a: 7,
            b: 19,
            c: "cool-cat".into(),
        },
        lua.load(chunk! {
            return {
                a = 7,
                b = 19,
                c = "cool-cat"
            }
        })
        .eval()
        .unwrap(),
    )];
    for (val, value) in expected {
        assert_eq!(SimpleStruct::from_lua(value.clone(), &lua).unwrap(), val);
        let tolua = val.to_lua(&lua).unwrap();
        assert_eq!(
            value.to_lua_string(&lua).unwrap(),
            tolua.to_lua_string(&lua).unwrap()
        );
    }
}
