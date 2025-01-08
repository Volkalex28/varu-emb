#![feature(cfg_version)]
#![feature(vec_pop_if)]
#![feature(iterator_try_collect)]
#![cfg_attr(not(version("1.80")), feature(lazy_cell))]
#![cfg_attr(not(version("1.79")), feature(associated_type_bounds))]

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse, Error, Result};

mod register;

fn implementation<D: ToTokens>(f: impl FnOnce() -> Result<D>) -> TokenStream {
    let out = match f() {
        Ok(out) => out.into_token_stream(),
        Err(err) => err.into_compile_error(),
    };
    out.into()
}

#[proc_macro_derive(Register, attributes(varuemb_devices))]
pub fn i2c(input: TokenStream) -> TokenStream {
    implementation::<register::Register>(|| parse(input))
}
