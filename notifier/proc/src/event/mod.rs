use crate::proc_meta_parser::Parser;
use proc_macro2::Ident;
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, DeriveInput, Error, Path};

pub struct Event<'a> {
    meta: &'a super::Meta,
    ident: &'a Ident,
    notif: Path,
    service: Option<Path>,
    is_mixer: bool,
}

impl<'a> Event<'a> {
    pub(crate) fn new(input: &'a DeriveInput, meta: &'a super::Meta) -> Result<Self, Error> {
        let mut attrs = input.attrs.iter().filter_map(|a| {
            if a.path().is_ident("notifier_event") {
                Some((a, false))
            } else if a.path().is_ident("notifier_mixer") {
                Some((a, true))
            } else {
                None
            }
        });
        let Some((attr, is_mixer)) = attrs.next() else {
            return Err(Error::new(
                input.span(),
                "Attribute \"notifier_event\" or \"notifier_mixer\" not found",
            ));
        };
        if let Some((attr, _)) = attrs.next() {
            return Err(Error::new(
                attr.span(),
                "Supports only one of \"notifier_event\" or \"notifier_mixer\" attributes",
            ));
        }

        let mut parser = Parser::new(["notifier", "service"], attr.span());
        attr.parse_nested_meta(|meta| parser.parse(meta))?;

        Ok(Self {
            meta,
            is_mixer,
            ident: &input.ident,
            notif: parser.get("notifier")?,
            service: match parser.get("service") {
                Ok(s) => Some(s),
                Err(_) if is_mixer => None,
                Err(err) => return Err(err),
            },
        })
    }
}

impl<'a> ToTokens for Event<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let _crate = &self.meta.crate_ident;
        let Self {
            ident,
            notif,
            service,
            is_mixer,
            ..
        } = self;

        let out = if *is_mixer {
            quote! {
                impl #_crate ::pubsub::mixer::Mixer<#notif> for #ident {}
            }
        } else {
            quote! {
                impl #_crate ::event::traits::Event<#notif> for #ident {
                    type Service = #service;
                }
            }
        };
        tokens.extend(out)
    }
}
