//! Contains the [`LuaToString`] trait.

/// Converts a `mlua::Value` into a string representation
///
/// This is useful for debugging
pub trait LuaToString {
    /// Convert this value into a string
    fn to_lua_string(&self, lua: &mlua::Lua) -> mlua::Result<String> {
        let mut buf = String::new();
        self.to_lua_string_with_buf(lua, &mut buf)?;
        Ok(buf)
    }
    /// Conver this value into a string, writing into the specified buffer
    fn to_lua_string_with_buf(&self, lua: &mlua::Lua, buf: &mut String) -> mlua::Result<()>;
}

impl LuaToString for mlua::Value<'_> {
    fn to_lua_string_with_buf(&self, lua: &mlua::Lua, buf: &mut String) -> mlua::Result<()> {
        use std::fmt::Write;
        match *self {
            mlua::Value::Nil => buf.push_str("nil"),
            mlua::Value::Boolean(b) => write!(buf, "{:?}", b).unwrap(),
            mlua::Value::LightUserData(light) => {
                write!(buf, "LightUserData({:?})", light.0).unwrap()
            }
            mlua::Value::Integer(val) => {
                write!(buf, "{}", val).unwrap();
            }
            mlua::Value::Number(val) => {
                write!(buf, "{}", val).unwrap();
            }
            mlua::Value::String(ref s) => {
                write!(buf, "{:?}", s.to_string_lossy()).unwrap();
            }
            mlua::Value::Table(ref tb) => {
                buf.push('{');
                let mut nextnumbered = 1;
                for r in tb.clone().pairs::<mlua::Value, mlua::Value>() {
                    let (k, v) = r?;
                    if matches!(k, mlua::Value::Integer(i) if i == nextnumbered)
                        || matches!(k, mlua::Value::Number(f) if f == (nextnumbered as f64))
                    {
                        nextnumbered += 1;
                    } else if let mlua::Value::String(ref sk) = k {
                        write!(buf, "{:?}=", sk.to_string_lossy()).unwrap();
                    } else {
                        buf.push('[');
                        k.to_lua_string_with_buf(lua, buf)?;
                        buf.push_str("]=");
                    }
                    v.to_lua_string_with_buf(lua, buf)?;
                    buf.push(',');
                }
                buf.push('}');
            }
            mlua::Value::Function(ref lf) => {
                // go with internal debug rpr
                write!(buf, "{:?}", lf).unwrap();
            }
            mlua::Value::Thread(ref lt) => {
                write!(buf, "{:?}", lt).unwrap();
            }
            mlua::Value::UserData(ref ud) => {
                write!(buf, "{:?}", ud).unwrap();
            }
            mlua::Value::Error(ref e) => {
                write!(buf, "{:?}", e).unwrap();
            }
        }
        Ok(())
    }
}
