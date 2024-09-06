#![feature(iterator_try_collect)]
#![feature(option_get_or_insert_default)]

use proc_macro::TokenStream as TS;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse, parse2, token, Attribute, Error, Result};
use syn_derive::Parse;

mod forward;

fn implementation<T: Parse + ToTokens>(input: TS) -> TS {
    let output = match parse::<T>(input) {
        Ok(data) => data.into_token_stream(),
        Err(err) => err.to_compile_error(),
    };
    output.into()
}

#[proc_macro_derive(Forward, attributes(varuemb))]
pub fn forward(input: TS) -> TS {
    self::implementation::<forward::Forward>(input)
}

#[derive(Debug, Parse)]
struct Parenthesized<D: parse::Parse> {
    #[allow(unused)]
    #[syn(parenthesized)]
    pub paren: token::Paren,
    #[syn(in = paren)]
    #[parse(Punctuated::parse_terminated)]
    pub data: Punctuated<D, token::Comma>,
}
impl<D: parse::Parse> Parenthesized<D> {
    fn parse_opt(input: parse::ParseStream) -> Result<Option<Self>> {
        if input.peek(token::Paren) {
            Parse::parse(input).map(Some)
        } else {
            Ok(None)
        }
    }
}

fn extract_attrs(attrs: &Vec<Attribute>, path: &[&'static str]) -> Result<TokenStream> {
    let mut list = attrs
        .iter()
        .filter_map(|attr| attr.path().is_ident("varuemb").then_some(&attr.meta))
        .cloned()
        .collect::<Vec<_>>();

    for ident in ["varuemb"].into_iter().chain(path.into_iter().copied()).take(path.len()) {
        list = extract(list, ident)?;
    }

    let ident = *path.last().unwrap();
    let attrs = extract::<TokenStream>(list, ident)?;
    return Ok(attrs
        .into_iter()
        .filter(|ts| !ts.is_empty())
        .flat_map(|mut ts| {
            ts.extend(quote::quote_spanned! { ts.span() => ,});
            ts
        })
        .collect());

    fn extract<T: Parse>(list: Vec<syn::Meta>, ident: &str) -> Result<Vec<T>> {
        list.into_iter().filter(|meta| meta.path().is_ident(ident)).try_fold(Vec::new(), |mut out, meta| {
            let syn::Meta::List(list) = meta else {
                return meta.require_list().map(|_| unreachable!());
            };
            out.push(parse2(list.tokens)?);
            Result::Ok(out)
        })
    }
}
