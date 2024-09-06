use crate::proc_meta_parser::Parser;
use heck::ToUpperCamelCase;
use linked_hash_map::LinkedHashMap;
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{spanned::Spanned, Attribute, Error, Expr, ItemImpl, LitBool, Meta, PatType, Type};

#[derive(Debug, Default)]
struct Data {
    alias: Option<Ident>,
    skipped: Option<LitBool>,
}

#[derive(Debug)]
pub struct SigBase {
    pub raw_ident: Ident,
    pub duration: Option<Expr>,
    pub input: Vec<PatType>,
    pub output: Option<Type>,
}

#[derive(Debug)]
pub struct InterfaceSignature {
    pub response: Option<TokenStream>,
    pub base: SigBase,
    pub block: TokenStream,
}

#[derive(Debug)]
pub struct Interface {
    pub skip: bool,
    pub alias: Option<Ident>,
    pub commands: Vec<InterfaceSignature>,
}
impl Default for Interface {
    fn default() -> Self {
        Interface {
            skip: true,
            alias: None,
            commands: Default::default(),
        }
    }
}
impl Interface {
    fn parse(attr: &Attribute) -> Result<Self, Error> {
        let mut parser = Parser::new(["alias", "impl"], attr.span());
        attr.parse_nested_meta(|meta| parser.parse(meta))?;

        let alias = parser.get("alias").ok();
        let skip = alias.is_none();
        let mut commands = Vec::default();
        let block = parser.get::<syn::Block>("impl")?;
        for item in block.stmts {
            let syn::Stmt::Item(syn::Item::Fn(item)) = item else {
                return Err(Error::new(item.span(), "Supported only fn definition"));
            };
            let sig = Self::make_sig(item)?;
            commands.push(sig);
        }
        Ok(Self {
            skip,
            alias,
            commands,
        })
    }

    fn make_sig(item: syn::ItemFn) -> Result<InterfaceSignature, Error> {
        let mut response = None;
        let mut duration = None;

        for attr in item.attrs {
            let attr_span = attr.path().span();
            let meta_span = attr.meta.span();
            if attr.path().is_ident("response") {
                let syn::Meta::List(syn::MetaList { tokens, .. }) = attr.meta else {
                    return Err(Error::new(meta_span, "Incorrect attribute"));
                };
                if response.is_some() {
                    return Err(Error::new(meta_span, "Response already exist"));
                }
                response = Some(tokens);
            } else if attr.path().is_ident("duration") {
                let syn::Meta::List(syn::MetaList { tokens, .. }) = attr.meta else {
                    return Err(Error::new(meta_span, "Incorrect attribute"));
                };
                if duration.is_some() {
                    return Err(Error::new(meta_span, "Duration already exist"));
                }
                duration = Some(syn::parse2::<Expr>(tokens)?);
            } else {
                return Err(Error::new(
                    attr_span,
                    "Support only \"response\" or \"duration\" attribute",
                ));
            }
        }
        let base = SigBase {
            raw_ident: item.sig.ident,
            duration,
            input: item
                .sig
                .inputs
                .into_iter()
                .filter_map(|param| match param {
                    syn::FnArg::Typed(
                        pat @ PatType {
                            pat: box syn::Pat::Ident(_),
                            ..
                        },
                    ) => Some(Ok(pat)),
                    syn::FnArg::Typed(ty) => Some(Err(Error::new(
                        ty.span(),
                        "Supported only \"name: Type\" signature",
                    ))),
                    syn::FnArg::Receiver(_) => None,
                })
                .try_collect()?,
            output: match item.sig.output {
                syn::ReturnType::Type(_, box ty) => Some(ty),
                syn::ReturnType::Default => None,
            },
        };
        Ok(InterfaceSignature {
            base,
            response,
            block: item.block.to_token_stream(),
        })
    }
}

#[derive(Debug)]
pub struct Signature {
    pub base: SigBase,
    pub interface: Interface,
}

pub struct ItemFn {
    attrs: Vec<Attribute>,
    sig: syn::Signature,
    #[allow(unused)]
    semi_token: syn::Token![;],
}

impl syn::parse::Parse for ItemFn {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let sig = input.parse()?;
        let semi_token = input.parse()?;
        Ok(Self {
            attrs,
            sig,
            semi_token,
        })
    }
}

#[derive(Debug)]
pub struct Parse {
    pub service: Type,
    pub handlers: LinkedHashMap<Ident, Signature>,
}

impl Parse {
    pub fn new(input: ItemImpl) -> Result<Self, Error> {
        let handlers = input
            .items
            .into_iter()
            .filter_map(|item| match item {
                syn::ImplItem::Fn(_fn) => Some(_fn.into_token_stream()),
                syn::ImplItem::Verbatim(tokens) => Some(tokens),
                _ => None,
            })
            .map(syn::parse2)
            .try_collect()?;
        Self::parse((*input.self_ty).clone(), handlers)
    }

    fn parse(service: Type, handlers: Vec<ItemFn>) -> Result<Self, Error> {
        let mut this = Self {
            service,
            handlers: Default::default(),
        };
        for ItemFn { attrs, mut sig, .. } in handlers.into_iter() {
            let mut data = LinkedHashMap::<String, (Data, Span)>::default();
            let mut no_response = None;
            let mut interface = None;
            let mut duration = None;
            for attr in attrs {
                if attr.path().is_ident("rpc_handler") {
                    if !matches!(attr.meta, Meta::List(_)) {
                        continue;
                    }
                    let mut parser = Parser::new(
                        ["alias", "response", "no_response", "duration"],
                        attr.span(),
                    );
                    attr.parse_nested_meta(|meta| parser.parse(meta))?;
                    if let Ok(alias) = parser.get("alias") {
                        sig.ident = alias;
                    }
                    if let Ok(ty) = parser.get::<Type>("response") {
                        sig.output =
                            syn::ReturnType::Type(syn::token::RArrow::default(), Box::new(ty));
                    }
                    no_response = parser.get("no_response").ok().as_ref().map(LitBool::value);
                    duration = parser.get("duration").ok();
                } else if attr.path().is_ident("rpc_handler_setup") {
                    let mut parser = Parser::new(["name", "skip", "alias"], attr.span());
                    attr.parse_nested_meta(|meta| parser.parse(meta))?;

                    let item_data = Data {
                        alias: parser.get("alias").ok(),
                        skipped: parser.get("skip").ok(),
                    };
                    if item_data.alias.is_some() || item_data.skipped.is_some() {
                        let name: Ident = parser.get("name")?;
                        if data
                            .insert(name.to_string(), (item_data, name.span()))
                            .is_some()
                        {
                            return Err(Error::new(attr.span(), "Parameter already settled"));
                        }
                    }
                } else if attr.path().is_ident("rpc_handler_interface") {
                    if interface.is_some() {
                        return Err(Error::new(attr.span(), "Interface already define"));
                    }
                    interface = Some(Interface::parse(&attr)?)
                }
            }
            let key = Ident::new(
                &sig.ident.to_string().to_upper_camel_case(),
                sig.ident.span(),
            );
            let sig = Signature {
                base: SigBase {
                    raw_ident: sig.ident,
                    duration,
                    input: sig
                        .inputs
                        .into_iter()
                        .filter_map(|param| match param {
                            syn::FnArg::Typed(
                                mut pat @ PatType {
                                    pat: box syn::Pat::Ident(_),
                                    ..
                                },
                            ) => {
                                let syn::Pat::Ident(ident) = pat.pat.as_mut() else {
                                    unreachable!()
                                };
                                if let Some((data, _)) = data.remove(&ident.ident.to_string()) {
                                    if data.skipped.map_or(false, |v| v.value) {
                                        return None;
                                    }
                                    if let Some(alias) = data.alias {
                                        ident.ident = alias;
                                    }
                                }
                                Some(Ok(pat.clone()))
                            }
                            syn::FnArg::Typed(ty) => Some(Err(Error::new(
                                ty.span(),
                                "Supported only \"name: Type\" signature",
                            ))),
                            syn::FnArg::Receiver(_) => None,
                        })
                        .try_collect()?,
                    output: match sig.output {
                        syn::ReturnType::Type(_, box ty) => Some(ty),
                        syn::ReturnType::Default => None,
                    }
                    .filter(|_| no_response.map_or(true, |v| !v)),
                },
                interface: interface.unwrap_or_default(),
            };
            this.handlers.insert(key, sig);
            if let Some((name, (_, span))) = data.front() {
                return Err(Error::new(*span, format!("Parameter {name}, not found")));
            }
        }
        Ok(this)
    }
}
