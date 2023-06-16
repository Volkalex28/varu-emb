use linked_hash_map::LinkedHashMap;
use quote::ToTokens;
use proc_macro2::TokenStream;
use syn::{Error, spanned::Spanned};

pub struct Parser {
    keys: Vec<String>,
    span: proc_macro2::Span,
    output: LinkedHashMap<String, TokenStream>,
}
impl Parser {
    pub fn new<'a, V: IntoIterator<Item = &'a T>, T: ToString + ?Sized + 'a>(keys: V, span: proc_macro2::Span) -> Self {
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
                let value = if to_parse.peek(syn::Token![&]) {
                    to_parse.parse::<syn::TypeReference>()?.into_token_stream()
                } else {
                    to_parse.parse::<syn::Expr>()?.into_token_stream()
                };
                // let to_parse_fork = to_parse.fork(); 
                // let mut value = to_parse.parse::<syn::Expr>()
                //     .map(|v| v.to_token_stream());
                // if value.is_err() {
                //     value = to_parse_fork.parse::<syn::Type>().map(|v| v.to_token_stream());
                //     // update buffer
                //     if value.is_ok() {
                //         _ = to_parse.parse::<syn::Type>();
                //     }
                // }
                if self.output.insert(key.clone(), value).is_some() {
                    return Err(syn::Error::new(meta.input.span(), format!("Key \"{key}\" already exist")));
                }
                return Ok(());
            }
        }
        Err(Error::new(
            meta.path.span(), 
            format!("Unsupported attribute value with key: {}", meta.path.into_token_stream().to_string())
        ))
    }

    pub fn get<T: syn::parse::Parse>(&mut self, key: impl ToString) -> Result<T, Error> {
        let key = key.to_string();
        let Some(tokens) = self.output.remove(&key) else { 
            return Err(Error::new(self.span, format!("Key \"{key}\" not found")));
        };
        Ok(syn::parse2(tokens).expect("get"))
    }
}
