use crate::Result;
use proc_macro2::TokenStream;
use syn::punctuated::Punctuated;
use syn::{parse, token, Error, Token};
use syn_derive::Parse;

use self::hw::Hw;
mod hw;

use self::peripheral::{Peripheral, PeripheralItem};
mod peripheral;

#[derive(Debug, Parse)]
struct Block {
    #[syn(braced)]
    _brace: token::Brace,

    #[syn(in = _brace)]
    #[parse(Self::parse_stmts)]
    stmts: Vec<syn::Stmt>,

    #[syn(in = _brace)]
    peripheral: Peripheral,

    #[syn(in = _brace)]
    hw: Hw,
}
impl Block {
    fn parse_stmts(input: parse::ParseStream) -> Result<Vec<syn::Stmt>> {
        let mut items = Vec::new();
        while !input.is_empty() && !input.peek(tokens::peripheral) && !input.peek(tokens::hw) {
            items.push(input.parse()?);
        }
        Ok(items)
    }
}

#[derive(Debug, Parse)]
struct Module {
    #[parse(syn::Attribute::parse_outer)]
    attrs: Vec<syn::Attribute>,

    ident: syn::Ident,
    _colon: Token![:],

    block: Block,
}
impl TryFrom<Module> for super::Module {
    type Error = Error;

    fn try_from(module: Module) -> Result<Self> {
        Ok(super::Module {
            attrs: module.attrs,
            ident: module.ident,
            peripheral: super::Spanned {
                span: module.block.peripheral._token.span,
                data: module.block.peripheral.items.into_iter().map(TryInto::try_into).try_collect()?,
            },
            hw: super::Spanned { span: module.block.hw._token.span, data: module.block.hw.fields },
            stmts: module.block.stmts,
        })
    }
}

#[derive(Debug, Parse)]
struct In {
    _in: Token![in],
    path: syn::Path,
}

#[derive(Debug, Parse)]
pub struct Hal {
    _for: Option<Token![for]>,

    path: syn::Path,

    #[parse(|input| input.peek(Token![in]).then(|| input.parse()).transpose())]
    _in: Option<In>,

    #[syn(braced)]
    _brace: token::Brace,

    #[syn(in = _brace)]
    #[parse(parse_vectored)]
    modules: Vec<Module>,
}
impl TryFrom<Hal> for super::Hal {
    type Error = Error;

    fn try_from(hal: Hal) -> Result<Self> {
        Ok(super::Hal {
            is_for: hal._for.is_some(),
            path: hal.path,
            _in: hal._in.map(|i| i.path),
            modules: hal.modules.into_iter().map(TryInto::try_into).try_collect()?,
        })
    }
}

mod tokens {
    syn::custom_keyword!(peripheral);
    syn::custom_keyword!(hw);
}

fn parse_vectored<T: parse::Parse>(input: parse::ParseStream) -> Result<Vec<T>> {
    let mut items = Vec::new();
    while !input.is_empty() {
        items.push(input.parse()?);
    }
    Ok(items)
}

impl TryFrom<PeripheralItem> for super::PeripheralItem {
    type Error = Error;

    fn try_from(mut item: PeripheralItem) -> Result<Self> {
        let cross = item
            .attrs
            .extract_if(|a| a.path().is_ident("cross"))
            .map(|a| -> Result<_> {
                let path = a.meta.require_list()?;
                path.parse_args::<TokenStream>()
            })
            .try_collect::<TokenStream>()?;

        Ok(super::PeripheralItem {
            attrs: item.attrs,
            cross: syn::parse2::<peripheral::cross::Cross>(cross)?.into(),
            ident: item.ident,
            bounds: item.bounds,
        })
    }
}

impl From<peripheral::cross::Cross> for super::PeripheralCross {
    fn from(cross: peripheral::cross::Cross) -> Self {
        use peripheral::cross::ErrorType;

        Self {
            config: cross.config.config,
            error: cross.error.map(|e| match e.ty {
                ErrorType::Eq { value, .. } => super::PeripheralCrossError::Type(value),
                ErrorType::Traits { value, .. } => super::PeripheralCrossError::Traits(value.into_iter().collect()),
            }),
        }
    }
}
