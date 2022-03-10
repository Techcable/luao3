//! Internal utilities for parsing
//!
//! These are not public and only intended for the macros.
#![allow(missing_docs)]

use std::fmt::Display;

use mlua::{FromLua, Lua, Value};


pub fn expect_table<'lua>(
    value: mlua::Value<'lua>,
    target_type: &'static str
) -> mlua::Result<mlua::Table<'lua>> {
    match value {
        mlua::Value::Table(tb) => Ok(tb),
        _ => Err(mlua::Error::FromLuaConversionError {
            from: value.type_name(),
            to: target_type,
            message: Some("Expected a table".into())
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TableKey {
    Str(&'static str),
    Number(u32),
}
impl<'lua> mlua::ToLua<'lua> for TableKey {
    fn to_lua(self, lua: &'lua Lua) -> mlua::Result<Value<'lua>> {
        match self {
            TableKey::Str(val) => val.to_lua(lua),
            TableKey::Number(val) => val.to_lua(lua)
        }
    }
}
impl Display for TableKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            TableKey::Str(s) => f.write_str(s),
            TableKey::Number(val) => write!(f, "{val}"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum EnumVariant {
    Named(String)
}

pub fn parse_enum_externally_tagged<'lua>(
    _lua: &'lua mlua::Lua,
    target_type: &'static str,
    lua_table: &mlua::Table<'lua>
) -> mlua::Result<(EnumVariant, mlua::Value<'lua>)> {
    let mut pairs = lua_table.clone()
        .pairs::<String, mlua::Value<'lua>>();
    let (len, res) = match pairs.next().transpose()? {
        Some((key, value)) => {
            (1 + pairs.count(), Some((EnumVariant::Named(key), value)))
        }
        None => (0, None)
    };
    if len == 1 {
        Ok(res.unwrap())
    } else {
        Err(mlua::Error::FromLuaConversionError {
            from: "table",
            to: target_type,
            message: Some(format!(
                "Externally tagged enum should have one field, but actually has {len}"
            ))
        })
    }

}
pub fn parse_field<'lua, T: FromLua<'lua>>(
    lua: &'lua mlua::Lua,
    target_type: &'static str,
    lua_table: &mlua::Table<'lua>,
    key: TableKey,
) -> mlua::Result<T> {
    let val: mlua::Value<'lua> = lua_table.get(key)?;
    let val_tp = val.type_name();
    T::from_lua(val, lua).map_err(|cause| {
        mlua::Error::FromLuaConversionError {
            from: val_tp,
            to: target_type,
            message: Some(format!("field {key}: {cause}"))
        }
    })
}