use enum_as_inner::EnumAsInner;
use heck::{ToShoutySnakeCase, ToSnakeCase};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use std::str::FromStr;
use std::{env, path};
use syn::spanned::Spanned;
use syn::{parse, Error, Token};
use syn_derive::{Parse, ToTokens};

type Result<T, E = Error> = std::result::Result<T, E>;
type Fields<A = Attribute> = HashMap<syn::Ident, A>;
type CfgMap = HashMap<String, Fields<syn::Expr>>;

#[derive(EnumAsInner, ToTokens)]
enum Attribute {
    Default(TokenStream),

    Link {
        #[to_tokens(|t, v| t.extend(quote! { #v ::__VARUEMB_CFG_LINK }) )]
        path: syn::Path,
    },
}
impl From<parsing::Attribute> for Attribute {
    fn from(attribute: parsing::Attribute) -> Self {
        match attribute.ty {
            parsing::AttributeType::Default { value, .. } => Self::Default(value),
            parsing::AttributeType::Link { path, .. } => Self::Link { path },
        }
    }
}

pub struct Cfg {
    path: path::PathBuf,
    toml: CfgMap,
    fields: Fields,
    input: syn::Ident,
}
impl Cfg {
    fn get_fields(input: syn::Fields) -> Result<Fields<Vec<syn::Attribute>>> {
        let syn::Fields::Named(fields) = input else {
            return Err(Error::new(input.span(), "Supported only named fields"));
        };
        let fields = fields.named.into_iter();
        let fields = fields.map(|f| (f.ident.unwrap(), f.attrs)).collect();
        Ok(fields)
    }

    fn parse_field(field: syn::Ident, attrs: Vec<syn::Attribute>) -> Result<<Fields as IntoIterator>::Item> {
        const MESSAGE: &'static str = "Attribute 'varuemb_cfg(default: ...)' or 'varuemb_cfg(link: ...)' not found on field";

        let attribute = attrs
            .into_iter()
            .find_map(|attr| attr.path().is_ident("varuemb_cfg").then(|| attr.parse_args::<parsing::Attribute>()))
            .unwrap_or_else(|| Err(Error::new(field.span(), MESSAGE)))?;

        Ok((field, attribute.into()))
    }

    fn parse_fields(input: syn::Fields) -> Result<Fields> {
        let fields = Self::get_fields(input)?;
        fields.into_iter().map(|(field, attrs)| Self::parse_field(field, attrs)).try_collect()
    }

    fn parse_offset(input: Vec<syn::Attribute>) -> Result<Option<String>> {
        let Some(attribute) = input
            .into_iter()
            .find_map(|attr| attr.path().is_ident("varuemb_cfg").then(|| attr.parse_args::<syn::LitStr>()))
            .transpose()?
        else {
            return Ok(None);
        };

        Ok(Some(attribute.value()))
    }

    fn apply_offset(
        offset: &mut dyn Iterator<Item = &str>,
        mut table: toml::Table,
    ) -> Option<Result<toml::Table, &'static str>> {
        let Some(current) = offset.next() else {
            return Some(Ok(table));
        };
        let toml::Value::Table(table) = table.remove(current)? else {
            return Some(Err("Must be a table"));
        };
        Self::apply_offset(offset, table)
    }
}
impl parse::Parse for Cfg {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let input = input.parse::<syn::ItemStruct>()?;

        let fields = Self::parse_fields(input.fields)?;

        let (loaded, path) = crate::load()?;
        let loaded = loaded.unwrap_or_default();
        let offset = Self::parse_offset(input.attrs)?;

        let mut toml = CfgMap::with_capacity(loaded.len());
        for (key, value) in loaded {
            let (entry, table) = if key.starts_with("cfg(") && key.ends_with(")") {
                let toml::Value::Table(table) = value else {
                    return Err(Error::new(Span::call_site(), format!("{key} must be a table")));
                };
                TokenStream::from_str(key.as_str())?;
                (toml.entry(key), table)
            } else {
                (toml.entry(String::new()), toml::Table::from_iter([(key, value)]))
            };

            match {
                let offset = offset.iter().flat_map(|s| s.split('.').map(|s| s.trim()));
                let mut offset = offset.filter(|s| !s.is_empty());

                Self::apply_offset(&mut offset, table)
            } {
                Some(Ok(table)) => {
                    let is_no_cfg = entry.key().is_empty();
                    let map = entry.or_default();
                    for (k, v) in table {
                        let Some((field, attr)) = fields.get_key_value(&syn::Ident::new(&k, Span::call_site())) else {
                            continue;
                        };
                        if is_no_cfg {
                            if v.is_table() && attr.is_default() {
                                return Err(Error::new(field.span(), "Attribute 'default' is not supported for table"));
                            }
                            if !v.is_table() && attr.is_link() {
                                return Err(Error::new(field.span(), "Attribute 'link' is not supported for not table"));
                            }
                        }
                        if !v.is_table() {
                            let expr = syn::parse_str(v.to_string().as_str())?;
                            map.insert(field.clone(), expr);
                        }
                    }
                }
                Some(Err(err)) => {
                    let message = format!(
                        "{pkg}.{key}.{offset}: {err}",
                        pkg = env::var("CARGO_PKG_NAME").as_ref().unwrap(),
                        key = entry.key(),
                        offset = offset.as_ref().unwrap()
                    );
                    return Err(Error::new(Span::call_site(), message));
                }
                None => continue,
            }
        }

        Ok(Self { path, toml, fields, input: input.ident })
    }
}
impl ToTokens for Cfg {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = &self.input;
        let cfg_path = format!("{}", self.path.display());
        let retrigger_ident =
            syn::Ident::new(&format!("__varuemb_cfg_{}__", ident.to_string().to_snake_case()), ident.span());
        let cfg_ident = syn::Ident::new(&ident.to_string().to_shouty_snake_case(), ident.span());

        let default = self.fields.iter().map(|(f, a)| quote! {#f : #a });
        let (loaded_default, loaded_cfgs) =
            self.toml
                .iter()
                .fold((TokenStream::new(), TokenStream::new()), |(mut default, mut cfgs), (cfg, data)| {
                    let cfg = (!cfg.is_empty()).then(|| {
                        let cfg = TokenStream::from_str(cfg).unwrap();
                        quote! {#[#cfg]}
                    });

                    if !data.is_empty() {
                        let data = data.iter().map(|(f, v)| quote! { __value . #f = #v });

                        if cfg.is_some() { &mut cfgs } else { &mut default }.extend(quote! {
                            #cfg
                            { #( #data ;)* }
                        });
                    }

                    (default, cfgs)
                });

        tokens.extend(quote! {
            pub const #cfg_ident : #ident = {
                #[allow(unused_mut)]
                let mut __value = #ident { #(#default,)* };

                #loaded_default
                #loaded_cfgs

                __value
            };

            impl #ident {
                #[allow(unused)]
                pub const __VARUEMB_CFG_LINK: Self = #cfg_ident;
            }

            mod #retrigger_ident {
                const _: &[u8] = include_bytes!(#cfg_path);
            }
        });
    }
}

mod tokens {
    syn::custom_keyword!(default);
    syn::custom_keyword!(link);
}

mod parsing {
    use super::*;

    #[derive(Parse, EnumAsInner)]
    pub enum AttributeType {
        #[parse(peek = tokens::default)]
        Default { _default: tokens::default, _colon: Token![:], value: TokenStream },
        #[parse(peek = tokens::link)]
        Link { _link: tokens::link, _colon: Token![:], path: syn::Path },
    }

    #[derive(Parse)]
    pub struct Attribute {
        pub ty: AttributeType,
    }
}
