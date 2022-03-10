//! Proc macros for luao3
use proc_macro::TokenStream as RawTokenStream;
use syn::parse_macro_input;

#[macro_use]
mod utils;
mod simple_module;
mod derive;
mod func;

#[proc_macro_derive(FromLua, attributes(lua))]
pub fn derive_from_lua(input: RawTokenStream) -> RawTokenStream {
    let derive = parse_macro_input!(input as syn::DeriveInput);
    match derive::fromlua::expand(derive) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.write_errors().into()
    }
}

#[proc_macro_attribute]
pub fn lua_function(attr: RawTokenStream, item: RawTokenStream) -> RawTokenStream {
    match func::expand(func::LuaFunctionMeta) {

    }
}

#[proc_macro]
pub fn declare_simple_module(input: RawTokenStream) -> RawTokenStream {
    let args = parse_macro_input!(input as simple_module::ModuleArgs);
    match simple_module::expand_module(args) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
