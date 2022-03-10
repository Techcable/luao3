use darling::{FromMeta, ToTokens};
use proc_macro::Punct;
use proc_macro2::TokenStream;
use syn::{parse_quote, FnArg, Token, PatType, punctuated::Punctuated};

#[derive(darling::FromMeta, Debug)]
pub struct LuaFunctionMeta {
    
}

macro_rules! require_matches {
    ($value:expr, $target:pat) => (
        require_matches!(
            $value, $target,
            format!("{} must not be {}", stringify!($value), stringify!($target))
        )
    );
    ($value:expr, $target:pat, $msg:expr) => {{
        let value = &$value;
        if !matches!(value, $target) {
            return Err(darling::Error::custom($msg).with_span(&value));
        }
    }};
}

/// Valid names for the lua parameter
const VALID_LUA_MARKER_NAMES: &'static [&str] = &["lua", "_lua"];
fn is_lua_marker_arg(arg: &syn::FnArg) -> bool {
    match *arg {
        syn::FnArg::Typed(ref tp) => {
            matches!(
                &*tp.pat, syn::Pat::Ident(ref pat) if
                    VALID_LUA_MARKER_NAMES.iter().any(|name| pat.ident == name)
            )
        },
        _ => false
    }
}


pub fn expand(meta: LuaFunctionMeta, item: syn::Item) -> Result<TokenStream, darling::Error> {
    let mut func = match item {
        syn::Item::Fn(func) => func,
        _ => return Err(darling::Error::custom("Expected a function item (`fn ...`)"))
    };
    // Rewrite the signature (that's 99% of what we do)
    let sig = &func.sig;
    require_matches!(sig.constness, None);
    require_matches!(sig.asyncness, None);
    require_matches!(sig.unsafety, None);
    require_matches!(sig.abi, None);
    require_matches!(sig.variadic, None);
    let syn::Signature {
        ref ident,
        ref generics,
        paren_token: _, fn_token,
        ref inputs,
        ..
    } = *sig;
    let mut rewritten_generics = generics.clone();
    if !rewritten_generics.lifetimes().any(|lt| lt.lifetime.ident == "lua") {
        rewritten_generics.params.push(parse_quote!('lua));
    }
    let mut original_arg_iter = inputs.iter().peekable();
    let mut rewritten_args: Punctuated<syn::FnArg, Token![,]> = Punctuated::new();
    if original_arg_iter.peek().copied().map_or(false, is_lua_marker_arg) {
        rewritten_args.push(original_arg_iter.next().unwrap().clone());
    } else {
        rewritten_args.push(parse_quote!(__lua: &'lua mlua::Lua))
    };
    let mut remaining_arg_types: Punctuated<syn::Type, Token![,]> = Punctuated::new();
    let mut remaining_arg_patterns: Punctuated<syn::Pat, Token![,]> = Punctuated::new();
    for remaining in original_arg_iter {
        match remaining {
            FnArg::Receiver(ref arg) => {
                return Err(darling::Error::custom("Unexpected reciever arg").with_span(arg));
            },
            FnArg::Typed(PatType {
                ty, pat,
                attrs: _, //TODO
                colon_token: _,
            }) => {
                remaining_arg_types.push((**ty).clone());
                remaining_arg_patterns.push((**pat).clone());
            }
        }
    }
    if !remaining_arg_patterns.empty_or_trailing() {
        remaining_arg_patterns.push_punct(Default::default());
    }

    if !remaining_arg_types.empty_or_trailing() {
        remaining_arg_types.push_punct(Default::default());
    }
    rewritten_args.push(parse_quote!((#remaining_arg_patterns): (#remaining_arg_types)));
    let rewritten_sig = syn::Signature {
        inputs: rewritten_args,
        generics: rewritten_generics,
        ..sig.clone()
    };
    func.sig = rewritten_sig;
    return Ok(func.into_token_stream());
}