use syn::{parse_quote, parse_quote_spanned, DeriveInput, spanned::Spanned};
use proc_macro2::{TokenStream, Ident, Span};
use darling::{FromDeriveInput};
use quote::{quote, quote_spanned};

#[derive(darling::FromField, Debug)]
pub struct ToLuaField {
    ident: Option<syn::Ident>,
}

trait FieldAccess {
    fn access(&self, member: syn::Member) -> Result<syn::Expr, darling::Error>;
}
struct SelfFieldAccess;
impl FieldAccess for SelfFieldAccess {
    fn access(&self, member: syn::Member) -> Result<syn::Expr, darling::Error> {
        Ok(parse_quote_spanned!(member.span() => self.#member))
    }
}
struct DestructureFieldAccess;
impl DestructureFieldAccess {
    pub fn destructure(
        &self,
        fields: &darling::ast::Fields<ToLuaField>
    ) -> Result<TokenStream, darling::Error> {
        Ok(match fields.style {
            darling::ast::Style::Unit => quote!(),
            darling::ast::Style::Tuple => {
                let field_names = (0..fields.fields.len()).map(|idx| {
                    Ident::new(&format!("field{}", idx), Span::call_site())
                });
                quote! {
                    (#(#field_names,)*)
                }
            },
            darling::ast::Style::Struct => {
                let field_names = fields.fields.iter()
                    .map(|fd| fd.ident.as_ref().unwrap());
                quote!({ #(#field_names,)* })
            }
        })
    }
}
impl FieldAccess for DestructureFieldAccess {
    fn access(&self, member: syn::Member) -> Result<syn::Expr, darling::Error> {
        let name: Ident = match member {
            syn::Member::Named(ref name) => name.clone(),
            syn::Member::Unnamed(ref idx) => {
                Ident::new(&format!("field{}", idx.index), idx.span)
            }
        };
        Ok(parse_quote!(#name))
    } 
}

impl ToLuaField {
    fn expand(&self, idx: u32, access: &dyn FieldAccess, lua_table_name: &Ident) -> Result<TokenStream, darling::Error> {
        let member: syn::Member = match self.ident {
            Some(ref name) => parse_quote!(#name),
            None => parse_quote!(#idx)
        };
        let key = match self.ident {
            Some(ref name) => quote_spanned!(name.span() => stringify!(#name)),
            None => quote!(#idx)
        };
        let access = access.access(member)?;
        Ok(quote! {
            #lua_table_name.set(#key, #access)?;
        })
    }
}

#[derive(darling::FromDeriveInput, Debug)]
pub struct ToLuaDerive {
    ident: syn::Ident,
    data: darling::ast::Data<ToLuaVariant, ToLuaField>,
}

#[derive(darling::FromVariant, Debug)]
pub struct ToLuaVariant {
    ident: Ident,
    fields: darling::ast::Fields<ToLuaField>
}

pub fn expand(input: DeriveInput) -> Result<TokenStream, darling::Error> {
    let derive = ToLuaDerive::from_derive_input(&input)?;
    let (_, ty_generics, where_clause)
        = input.generics.split_for_impl();
    let mut impl_generics = input.generics.clone();
    if !impl_generics.params.iter().any(|param| {
        matches!(param, syn::GenericParam::Lifetime(ref lt)
            if lt.lifetime.ident == "lua")
    }) {
        impl_generics.params.push(parse_quote!('lua));
    }
    let (impl_generics, _, _) = impl_generics.split_for_impl();
    let original_name = &derive.ident;
    let handle_unit_variants: Option<TokenStream> = if let darling::ast::Data::Enum(ref variants) = derive.data {
        let mut match_unit_variants = variants.iter()
            .filter(|var| var.fields.is_unit())
            .map(|var| {
                let text = var.ident.to_string();
                let ident = &var.ident;
                quote!(#original_name::#ident => return Ok(#text.to_lua(lua)?))
            })
            .peekable();
        if match_unit_variants.peek().is_some() {
            Some(quote! {
                match self {
                    #(#match_unit_variants,)*
                    _ => {} // fallthrough
                }
            })
        } else {
            None
        }
    } else {
        None
    };
    let conversion_impl = match derive.data {
        darling::ast::Data::Struct(ref fields) => {
            expand_variant_into(
                &SelfFieldAccess,
                fields,
                parse_quote!(lua_table)
            )?
        },
        darling::ast::Data::Enum(ref variants) => {
            /*
             * TODO: We need a better way to differentiate enum variants
             *
             * This is essentially the serde "Externally tagged" enum representation:
             * https://serde.rs/enum-representations.html#externally-tagged
             */
            let variant_matches = variants.iter()
                .filter(|var| !var.fields.is_unit())
                .map(|var| {
                    let expanded = expand_variant_into(
                        &DestructureFieldAccess,
                        &var.fields,
                        parse_quote!(nested_table)
                    )?;
                    let variant_name = &var.ident;
                    let destructure = DestructureFieldAccess.destructure(&var.fields)?;
                    Ok(quote!(#original_name::#variant_name #destructure => {
                        let nested_table = lua.create_table()?;
                        lua_table.set(stringify!(#variant_name), nested_table)?;
                        #expanded
                    }))
               }).collect::<Result<Vec<_>, darling::Error>>()?;
            quote! {
                match &**self {
                    #(#variant_matches)*
                    _ => return Err(mlua::Error::FromLuaConversionError {
                        from: val_tp,
                        to: target_type,
                        // NOTE: Unit variants are parsed as strings
                        message: Some(format!("Unknown variant name: {variant_name}"))
                    })
                }
            }
        }
    };
    Ok(quote! {
        impl #impl_generics mlua::ToLua<'lua> for #original_name #ty_generics #where_clause {
            fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
                let type_name: &'static str = std::any::type_name::<#original_name #ty_generics>();
                #handle_unit_variants
                let lua_table = lua.create_table()?;
                #conversion_impl
                Ok(mlua::Value::Table(lua_table))
            }
        }
    })
}

fn expand_variant_into(
    access: &dyn FieldAccess,
    fields: &darling::ast::Fields<ToLuaField>,
    lua_table_name: Ident
) -> Result<TokenStream, darling::Error> {
    if matches!(fields.style, darling::ast::Style::Unit) {
        return Ok(quote!());
    }
    let stmts = fields.iter().enumerate()
        .map(|(idx, fd)| {
            fd.expand(idx as u32, access, &lua_table_name)
        })
        .collect::<Result<Vec<_>, darling::Error>>()?;
    Ok(quote!(#(#stmts)*))

}
