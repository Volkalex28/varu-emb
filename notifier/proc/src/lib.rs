#![feature(box_patterns)]
#![feature(iterator_try_collect)]
#![feature(const_trait_impl)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::Error;

mod event;
mod notifier;
mod rpc;
mod service;

#[derive(Debug)]
struct Meta {
    crate_ident: TokenStream2,
}
impl Default for Meta {
    fn default() -> Self {
        Self {
            crate_ident: quote!(::varuemb::notifier),
        }
    }
}

/// This is a procedural macro that provides auto-generation of the "Notifier" structure
/// for storing and managing communication channels between services
#[proc_macro_attribute]
pub fn notifier(attrs: TokenStream, input: TokenStream) -> TokenStream {
    (|| -> Result<TokenStream2, Error> {
        let meta = Meta::default();
        let input = syn::parse(input)?;
        let notif = notifier::Notifier::new(attrs.into(), &input, &meta)?;
        Ok(notif.to_token_stream())
    })()
    .unwrap_or_else(|err| err.to_compile_error())
    .into()
}

#[proc_macro_derive(
    Service,
    attributes(
        notifier_service,
        notifier_publisher,
        notifier_subscriber,
        notifier_rpc_subscriber
    )
)]
pub fn service_derive(input: TokenStream) -> TokenStream {
    (|| -> Result<TokenStream2, Error> {
        let meta = Meta::default();
        let input = syn::parse(input)?;
        let service = service::Service::new(&input, &meta)?;
        Ok(service.to_token_stream())
    })()
    .unwrap_or_else(|err| err.to_compile_error())
    .into()
}

#[proc_macro_attribute]
pub fn rpc_handlers(attrs: TokenStream, input: TokenStream) -> TokenStream {
    (|attrs: TokenStream2| -> Result<TokenStream2, Error> {
        let meta = Meta::default();
        let attr = syn::parse_quote!(#[rpc_handlers(#attrs)]);
        let input = syn::parse(input)?;
        let rpc = rpc::Rpc::new(&attr, input, &meta)?;
        Ok(rpc.to_token_stream())
    })(attrs.into())
    .unwrap_or_else(|err| err.to_compile_error())
    .into()
}

#[proc_macro_derive(Event, attributes(notifier_event, notifier_mixer))]
pub fn event_derive(input: TokenStream) -> TokenStream {
    (|| -> Result<TokenStream2, Error> {
        let meta = Meta::default();
        let input = syn::parse(input)?;
        let event = event::Event::new(&input, &meta)?;
        Ok(event.to_token_stream())
    })()
    .unwrap_or_else(|err| err.to_compile_error())
    .into()
}
