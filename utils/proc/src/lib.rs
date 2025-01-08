#![feature(extract_if)]
#![feature(iterator_try_collect)]

use proc_macro::TokenStream as TS;
use quote::ToTokens;
use syn::parse;
use syn::parse::Parse;

mod cfg_derivable_item;
mod cfg_impl_block;
mod cfg_trait_bound;
mod cfg_type_alias;
mod multi_impl_block;

fn implementation<T: Parse + ToTokens>(input: TS) -> TS {
    let output = match parse::<T>(input) {
        Ok(data) => data.into_token_stream(),
        Err(err) => err.to_compile_error(),
    };
    output.into()
}

#[proc_macro]
pub fn multi_impl_block(input: TS) -> TS {
    self::implementation::<multi_impl_block::MultiImplBlock>(input)
}

#[proc_macro_attribute]
pub fn cfg_derivable_item(attrs: TS, input: TS) -> TS {
    self::implementation::<cfg_derivable_item::CfgDerivable>(attrs.into_iter().chain(input.into_iter()).collect())
}

#[proc_macro_attribute]
pub fn cfg_impl_block(attrs: TS, input: TS) -> TS {
    self::implementation::<cfg_impl_block::CfgImplBlock>(attrs.into_iter().chain(input.into_iter()).collect())
}

#[proc_macro_attribute]
pub fn cfg_trait_bound(_: TS, input: TS) -> TS {
    self::implementation::<cfg_trait_bound::CfgTraitBound>(input)
}

#[proc_macro_attribute]
pub fn cfg_type_alias(attrs: TS, input: TS) -> TS {
    self::implementation::<cfg_type_alias::CfgTypeAlias>(attrs.into_iter().chain(input.into_iter()).collect())
}
