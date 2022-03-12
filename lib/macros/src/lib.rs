//! Proc macros for luao3
use darling::FromMeta;
use func::LuaFunctionMeta;
use proc_macro::TokenStream as RawTokenStream;
use syn::parse_macro_input;

#[macro_use]
mod utils;
mod derive;
mod func;
mod simple_module;

#[proc_macro_derive(FromLua, attributes(lua))]
pub fn derive_from_lua(input: RawTokenStream) -> RawTokenStream {
    let derive = parse_macro_input!(input as syn::DeriveInput);
    match derive::fromlua::expand(derive) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.write_errors().into(),
    }
}

#[proc_macro_derive(ToLua, attributes(lua))]
pub fn derive_to_lua(input: RawTokenStream) -> RawTokenStream {
    let derive = parse_macro_input!(input as syn::DeriveInput);
    match derive::tolua::expand(derive) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.write_errors().into(),
    }
}

#[proc_macro_attribute]
pub fn lua_function(args: RawTokenStream, item: RawTokenStream) -> RawTokenStream {
    let args = parse_macro_input!(args as syn::AttributeArgs);
    let item = parse_macro_input!(item as syn::Item);
    match LuaFunctionMeta::from_list(&*args).and_then(|meta| func::expand(meta, item)) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.write_errors().into(),
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
