use linked_hash_map::LinkedHashMap;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{spanned::Spanned, Error};

pub struct Parser {
    keys: Vec<String>,
    span: proc_macro2::Span,
    output: LinkedHashMap<String, TokenStream>,
}
impl Parser {
    pub fn new<'a, V: IntoIterator<Item = &'a T>, T: ToString + ?Sized + 'a>(
        keys: V,
        span: proc_macro2::Span,
    ) -> Self {
        Self {
            span,
            output: Default::default(),
            keys: keys.into_iter().map(ToString::to_string).collect(),
        }
    }

    pub fn parse(&mut self, meta: syn::meta::ParseNestedMeta) -> syn::parse::Result<()> {
        for key in self.keys.iter() {
            if meta.path.is_ident(key) {
                let to_parse = meta.value()?;
                
                let value = if to_parse.fork().parse::<syn::Expr>().is_ok() {
                    to_parse.parse::<syn::Expr>()?.into_token_stream()
                } else {
                    to_parse.parse::<syn::Type>()?.into_token_stream()
                };
                if self.output.insert(key.clone(), value).is_some() {
                    return Err(syn::Error::new(
                        meta.input.span(),
                        format!("Attribute '{key}' already exist"),
                    ));
                }
                return Ok(());
            }
        }
        Err(Error::new(
            meta.path.span(),
            format!(
                "Unsupported value for '{}' attribute",
                meta.path.into_token_stream().to_string()
            ),
        ))
    }

    pub fn get<T: syn::parse::Parse>(&mut self, key: impl ToString) -> Result<T, Error> {
        let key = key.to_string();
        let Some(tokens) = self.output.remove(&key) else { 
            return Err(Error::new(self.span, format!("Attribute '{key}' not found")));
        };
        syn::parse2(tokens)
    }
}
