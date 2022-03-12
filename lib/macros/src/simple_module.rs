use indexmap::map::Entry;
use indexmap::IndexMap;
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_kwargs::{parse_macro_arg_via_syn, MacroKeywordArgs};
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{braced, parse_quote_spanned, Error, Expr, Path, Token};

use crate::utils;

#[derive(MacroKeywordArgs)]
pub struct ModuleArgs {
    name: Ident,
    members: ModuleMembers,
}

pub fn expand_module(args: ModuleArgs) -> Result<TokenStream, Error> {
    let name = &args.name;
    let reserved_names = &["lua", "res"];
    let members_decl = utils::collect_vec_combining_errors(
        args.members.items.iter().map(|(name, item)| {
            let declared = item.item.declare(&|ident: &Ident| {
                reserved_names.iter().any(|reserved| ident == *reserved)
            })?;
            let name_expr = name.string_expr();
            Ok(quote!(res.set(
                #name_expr,
                #declared
            )))
        }),
        utils::combine_syn_errors,
    )?;
    Ok(quote! {
        fn #name<'lua>(lua: &'lua mlua::Lua) -> mlua::Result<mlua::Table<'lua>> {
            let res = lua.create_table()?;
            #(#members_decl ;)*
            Ok(res)
        }
    })
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum LuaName {
    Ident(Ident),
    Text(syn::LitStr),
}

impl LuaName {
    fn string_expr(&self) -> Expr {
        match *self {
            LuaName::Ident(ref i) => {
                parse_quote_spanned!(i.span() => stringify!(#i))
            }
            LuaName::Text(ref lit) => {
                syn::parse_quote!(#lit)
            }
        }
    }
}

impl Spanned for LuaName {
    fn span(&self) -> Span {
        match *self {
            LuaName::Ident(ref id) => id.span(),
            LuaName::Text(ref lit) => lit.span(),
        }
    }
}
impl Parse for LuaName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Ident) {
            Ok(LuaName::Ident(input.parse::<syn::Ident>()?))
        } else if input.peek(syn::LitStr) {
            Ok(LuaName::Text(input.parse::<syn::LitStr>()?))
        } else {
            Err(input.error("Expected a lua name (either identifier or string)"))
        }
    }
}

pub struct ModuleMembers {
    items: IndexMap<LuaName, ModuleItemDecl>,
}
parse_macro_arg_via_syn!(ModuleMembers);
impl Parse for ModuleMembers {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let items: ParseBuffer;
        braced!(items in input);
        let items: Punctuated<ModuleItemDecl, Token![,]> = Punctuated::parse_terminated(&items)?;
        let mut map: IndexMap<LuaName, ModuleItemDecl> = IndexMap::with_capacity(items.len());
        let mut errors: Vec<syn::Error> = Vec::new();
        for item in items {
            error_loop!(errors, Result<(), syn::Error>; {
                let name = item.name()?;
                match map.entry(name.clone()) {
                    Entry::Occupied(_) => {
                        return Err(Error::new(
                            name.span(),
                            "Conflicts with existing entry"
                        ));
                    },
                    Entry::Vacant(e) => {
                        e.insert(item);
                    }
                }
                Ok(())
            })
        }
        utils::combine_syn_errors(errors)?;
        Ok(ModuleMembers { items: map })
    }
}
struct ModuleItemDecl {
    custom_name: Option<LuaName>,
    item: ModuleItem,
}

impl ModuleItemDecl {
    pub fn name(&self) -> Result<LuaName, Error> {
        self.custom_name
            .clone()
            .or_else(|| self.item.implicit_name().cloned().map(LuaName::Ident))
            .ok_or_else(|| syn::Error::new(self.item.decl_span(), "Can't detect implicit name"))
    }
}

impl Parse for ModuleItemDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ModuleItem = input.parse()?;
        let custom_name = if input.peek(Token![as]) {
            input.parse::<Token![as]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(ModuleItemDecl { item, custom_name })
    }
}

enum ModuleItem {
    Function { path: Path },
}
impl ModuleItem {
    pub fn declare(&self, reserved: &dyn Fn(&Ident) -> bool) -> Result<TokenStream, Error> {
        let qualify = |p: &Path| {
            if let Some(ident) = p.get_ident().filter(|i| reserved(*i)) {
                syn::parse_quote!(self::#ident)
            } else {
                p.clone()
            }
        };
        match *self {
            ModuleItem::Function { ref path } => {
                let path = qualify(path);
                Ok(quote_spanned! { path.span() =>
                    lua.create_function(#path)?
                })
            }
        }
    }
    #[inline]
    fn implicit_name(&self) -> Option<&Ident> {
        match *self {
            ModuleItem::Function { ref path, .. } => {
                // Pretty sure path must be nonempty
                Some(&path.segments.last().unwrap().ident)
            }
        }
    }
    #[inline]
    fn decl_span(&self) -> Span {
        match *self {
            ModuleItem::Function { ref path, .. } => path.span(),
        }
    }
}

impl Parse for ModuleItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![fn]) {
            input.parse::<Token![fn]>()?;
            let path = input.parse::<Path>()?;
            Ok(ModuleItem::Function { path })
        } else {
            Err(input.error("Unexpected token for module item"))
        }
    }
}
