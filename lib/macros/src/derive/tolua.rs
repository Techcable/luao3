use syn::{parse_quote, parse_quote_spanned, DeriveInput, spanned::Spanned};
use proc_macro2::{TokenStream, Ident, Span};
use quote::{quote, quote_spanned};

#[derive(darling::FromField, Debug)]
pub struct ToLuaField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
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
        fields: darling::ast::Fields<ToLuaField>
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
                quote!({ #(ref #field_names,)* })
            }
        })
    }
}
impl FieldAccess for DestructureFieldAccess {
    fn access(&self, member: syn::Member) -> Result<syn::Expr, darling::Expr> {
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
    pub fn expand(&self, idx: u32, access: &dyn FieldAccess) -> Result<TokenStream, darling::Error> {
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
            lua_table.set(#key, &#access)?;
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
            expand_variant(
                &SelfFieldAccess,
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
        impl #impl_generics mlua::FromLua<'lua> for #original_name #ty_generics #where_clause {
            fn from_lua(lua_value: mlua::Value<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
                const TYPE_NAME: &'static str = std::any::type_name::<#original_name #ty_generics>();
                #handle_unit_variants
                let lua_table = luao3::parse_helpers::expect_table(lua_value, TYPE_NAME)?;
                Ok(#conversion_impl)
            }
        }
    })
}

fn expand_variant(
    access: &dyn FieldAccess,
    variant_name: Ident,
    fields: &darling::ast::Fields<FromLuaField>
) -> Result<TokenStream, darling::Error> {
    todo!()
}