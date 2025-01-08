#![feature(cfg_version)]
#![feature(vec_pop_if)]
#![feature(iterator_try_collect)]
#![cfg_attr(not(version("1.79")), feature(associated_type_bounds))]

use proc_macro::TokenStream as TS;
use syn::{parse, Result};
use syn_derive::Parse;

mod execution;
mod task;

#[proc_macro_derive(Task, attributes(varuemb_executor))]
pub fn task(input: TS) -> TS {
    implementation::<task::Task>(input)
}

#[proc_macro_derive(ExecutionMeta, attributes(varuemb_executor))]
pub fn execution(input: TS) -> TS {
    implementation::<execution::Execution>(input)
}

fn implementation<T: parse::Parse + quote::ToTokens>(input: TS) -> TS {
    let output = match parse::<T>(input) {
        Ok(data) => data.into_token_stream(),
        Err(err) => err.to_compile_error(),
    };
    output.into()
}

#[derive(Debug, Parse)]
struct ParenAttribute<T: parse::Parse, C: parse::Parse> {
    _token: T,

    #[syn(parenthesized)]
    _paren: syn::token::Paren,

    #[syn(in = _paren)]
    content: C,
}

#[derive(Debug, Parse)]
struct ValueAttribute<T: parse::Parse, V: parse::Parse> {
    _token: T,
    _colon: syn::token::Colon,
    value: V,
}
