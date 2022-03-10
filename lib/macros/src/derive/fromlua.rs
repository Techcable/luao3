use darling::ast::{Style};
use darling::FromDeriveInput;
use proc_macro2::{TokenStream, Ident};
use quote::{quote, quote_spanned};
use syn::{DeriveInput, parse_quote, parse_quote_spanned, spanned::Spanned};

#[derive(darling::FromField, Debug)]
pub struct FromLuaField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    #[darling(default, rename = "default")]
    is_default: darling::util::Flag,
}

impl FromLuaField {
    pub fn expand(&self, idx: u32) -> TokenStream {
        let key = match self.ident {
            Some(ref name) => {
                quote!(luao3::parse_helpers::TableKey::String(stringify!(#name)))
            },
            None => quote!(luao3::parse_helpers::TableKey::Number(#idx)),
        };
        let ty = &self.ty;
        let conversion_ty = if self.is_default.is_some() {
            parse_quote_spanned!(ty.span() => Option<#ty>)
        } else {
            ty.clone()
        };
        let primary_conversion = quote! {
            luao3::parse_helpers::parse_field::<#conversion_ty>(
                lua, TYPE_NAME,
                &lua_table, #key
            )?
        };
        if self.is_default.is_some() {
            let def = quote_spanned!(ty.span() => <#ty as Default>::default);
            quote!((#primary_conversion).unwrap_or_else(#def))
        } else {
            primary_conversion
        }
    }
}

#[derive(darling::FromDeriveInput, Debug)]
pub struct FromLuaDerive {
    ident: syn::Ident,
    data: darling::ast::Data<FromLuaVariant, FromLuaField>,
}

#[derive(darling::FromVariant, Debug)]
pub struct FromLuaVariant {
    ident: Ident,
    fields: darling::ast::Fields<FromLuaField>
}

pub fn expand(input: DeriveInput) -> Result<TokenStream, darling::Error> {
    let derive = FromLuaDerive::from_derive_input(&input)?;
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
                quote!(#text => return Ok(#original_name::#ident))
            })
            .peekable();
        if match_unit_variants.peek().is_some() {
            Some(quote! {
                if let mlua::Value::String(ref s) = lua_value {
                    // TODO: Give more descriptive error if UTF8 conversion fails
                    let variant_name = variant_name.to_str()?;
                    match s {
                        #(#match_unit_variants,)*
                        _ => {}
                    }
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
            expand_variant(
                fields.style,
                original_name.clone(),
                fields
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
                    let expand = expand_variant(
                        var.fields.style,
                        var.ident.clone(),
                        &var.fields
                    )?;
                    let name = var.ident.to_string();
                    Ok(quote!(#name => #original_name::#expand))
               }).collect::<Result<Vec<_>, darling::Error>>()?;
            quote! {
                // TODO: Better error messages (consider both unit variants and regular enum variants)
                let (variant, value) = luao3::parse_helpers::parse_enum_externally_tagged(
                    lua,
                    TYPE_NAME,
                    lua_table,
                )?;
                let variant_name = match variant {
                    luao3::parse_helpers::EnumVariant::Named(ref name) => name
                };
                match &**variant_name {
                    #(#variant_matches)*
                    _ => Err(mlua::Error::FromLuaConversionError {
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
        impl #impl_generics mlua::FromLua<'lua> for #original_name #ty_generics #where_clause {
            fn from_lua(lua_value: mlua::Value<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
                const TYPE_NAME: &'static str = std::any::type_name::<#original_name #ty_generics>();
                #handle_unit_variants
                let lua_table = luao3::parse_helpers::expect_table(lua, TYPE_NAME);
                Ok(#conversion_impl)
            }
        }
    })
}

fn expand_variant(
    _style: Style,
    variant_name: Ident,
    fields: &darling::ast::Fields<FromLuaField>
) -> Result<TokenStream, darling::Error> {
    let field_conversions = fields.fields.iter().enumerate()
        .map(|(idx, fd)| fd.expand(idx as u32));
    let field_names = fields.fields.iter()
        .map(|fd| fd.ident.as_ref().unwrap());
    Ok(match fields.style {
        Style::Tuple => {
            quote!(#variant_name(#(#field_conversions,)*))
        }
        Style::Struct => {
            quote!(#variant_name {
                #(#field_names : #field_conversions,)*
            })
        }
        Style::Unit => quote!(#variant_name)
    })
}