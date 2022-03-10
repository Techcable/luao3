use darling::FromMeta;
use proc_macro2::TokenStream;

#[derive(darling::FromMeta, Debug)]
pub struct LuaFunctionMeta;

macro_rules! require_matches {
    ($value:expr, $target:pat) => (
        require_matches!(
            $value, $target,
            format!("{} must not be {}", stringify!($value), stringify!($target))
        )
    );
    ($value:expr, $target:pat, $msg:expr) => {{
        let value = $value;
        if !matches!(value, $target) {
            return Err(darling::Error::custom($msg).with_span(&value));
        }
    }};
}

/// Valid names for the lua parameter
const VALID_LUA_MARKER_NAMES: &'static [&str] = &["lua", "_lua"];

pub fn expand(meta: LuaFunctionMeta, item: syn::Item) -> Result<TokenStream, darling::Error> {
    let mut func = match item {
        syn::Item::Fn(ref func) => func,
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
        ref output,
        ..
    } = *sig;
    let arg_iter = inputs.iter().peekable();
    let mut rewritten_args = syn::Punctuated::new();
    if let Some(lua_arg) = arg_iter.peek().filter(|arg| matches!(arg, FnArg::Typed(syn::PatTy {
        ref ty
    }))) {
        rewritten_args.push(lua_arg);
    } else {
        rewritten_args.push(parse_quote!(__lua: &mlua::Lua))
    };
    let rewritten_sig = syn::Signature {
        
        ..sig.clone()
    }
    Ok(parse_quote_spanned! {
        fn 
    })
}