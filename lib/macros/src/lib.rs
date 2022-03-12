//! Proc macros for luao3
use std::fmt::Display;

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
    let name = derive.ident.clone();
    let tk = match derive::fromlua::expand(derive) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.write_errors().into(),
    };
    maybe_debug("FromLua", &name, tk, MacroCtx::File)
}

#[proc_macro_derive(ToLua, attributes(lua))]
pub fn derive_to_lua(input: RawTokenStream) -> RawTokenStream {
    let derive = parse_macro_input!(input as syn::DeriveInput);
    let name = derive.ident.clone();
    let tk = match derive::tolua::expand(derive) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.write_errors().into(),
    };
    maybe_debug("ToLua", &name, tk, MacroCtx::File)
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

#[derive(Copy, Clone, Debug)]
enum MacroCtx {
    File,
}
impl MacroCtx {
    pub fn format_file(&self, input: RawTokenStream) -> Result<String, syn::Error> {
        let MacroCtx::File = *self;
        Ok(prettyplease::unparse(&syn::parse(input)?))
    }
}
fn maybe_debug(
    macro_name: &str,
    arg: &dyn Display,
    res: RawTokenStream,
    ctx: MacroCtx,
) -> RawTokenStream {
    let matches = if let Ok(debug) = std::env::var("DEBUG_MACRO") {
        if let Some(remaining) = debug.strip_prefix(macro_name) {
            let arg = format!("{}", arg);
            if let Some(remaining) = remaining.strip_prefix(':') {
                remaining.starts_with(&arg)
            } else {
                true
            }
        } else {
            false
        }
    } else {
        false
    };
    if matches {
        eprintln!("{} for {}:", macro_name, arg);
        match ctx.format_file(res.clone()) {
            Ok(fmt) => {
                eprintln!("{}", fmt)
            }
            Err(_) => {
                eprintln!("{}", res);
            }
        }
    }
    res
}
