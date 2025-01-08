#![feature(iterator_try_collect)]
#![feature(cfg_version)]
#![feature(extract_if)]
#![cfg_attr(not(version("1.83")), feature(option_get_or_insert_default))]

use proc_macro::TokenStream as TS;
use quote::ToTokens;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{parse, token, Error, Result};
use syn_derive::Parse;

mod forward;
mod hal;
// mod hal2;

fn implementation<T: Parse + ToTokens>(input: TS) -> TS {
    let output = match parse::<T>(input) {
        Ok(data) => data.into_token_stream(),
        Err(err) => err.to_compile_error(),
    };
    output.into()
}

#[proc_macro_derive(Forward, attributes(varuemb_cross))]
pub fn forward(input: TS) -> TS {
    self::implementation::<forward::Forward>(input)
}

#[proc_macro]
pub fn hal(input: TS) -> TS {
    self::implementation::<hal::Hal>(input)
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
