use crate::proc_meta_parser::Parser;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, Attribute, Error, Expr, Ident, ItemImpl, LitBool, Path, Type};

pub mod parse;

pub struct Rpc<'a> {
    meta: &'a super::Meta,
    parse: parse::Parse,
    request: (Ident, bool),
    response: (Ident, bool),
    error: Option<Type>,
    notif: Path,
}

impl<'a> Rpc<'a> {
    pub(crate) fn new(
        attr: &'a Attribute,
        input: ItemImpl,
        meta: &'a super::Meta,
    ) -> Result<Self, Error> {
        let mut parser = Parser::new(
            [
                "notifier",
                "request",
                "response",
                "error",
                "no_debug_request",
                "no_debug_response",
            ],
            attr.span(),
        );
        attr.parse_nested_meta(|meta| parser.parse(meta))
            .expect("Parse M");

        let parse = parse::Parse::new(input)?;
        let this = Self {
            meta,
            parse,
            request: (
                parser.get("request")?,
                parser
                    .get::<LitBool>("no_debug_request")
                    .map_or(true, |v| !v.value()),
            ),
            response: (
                parser.get("response")?,
                parser
                    .get::<LitBool>("no_debug_response")
                    .map_or(true, |v| !v.value()),
            ),
            notif: parser.get("notifier")?,
            error: parser.get("error").ok(),
        };
        Ok(this)
    }
}

impl ToTokens for Rpc<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let _crate = &self.meta.crate_ident;
        let notif = &self.notif;
        let service = &self.parse.service;
        let _impl = quote!(__rpc_impl);
        let (req, req_dbg) = &self.request;
        let (resp, resp_dbg) = &self.response;
        let err = &self.error;

        #[rustfmt::skip]
        let generate_fn = |
            ident: &Ident,
            raw_ident: &Ident,
            parse::SigBase { input, output, .. }: &_,
            not_default: Option<&TokenStream>,
            response: Option<&TokenStream>,
            duration: Option<&Expr>
        | {
            let inputs = input
                .iter()
                .flat_map(|p| {
                    let attrs = &p.attrs;
                    let ident = &p.pat;
                    quote!(#(#attrs)* #ident ,)
                })
                .collect::<TokenStream>();
            let req = if let Some(block) = not_default {
                quote!(#block)
            } else {
                quote! (#req :: #ident { #inputs })
            };
            let resp_map = if let Some(resp) = response {
                quote!((#resp)(__ret))
            } else {
                quote!(::core::option::Option::Some(__ret))
            };
            let duration = if let Some(duration) = duration {
                quote!(::core::option::Option::Some(#_crate ::Duration::from_millis(#duration)))
            } else {
                quote!(::core::option::Option::Some(#_crate ::Duration::from_secs(10)))
            };
            if let Some(output) = output.as_ref() {
                quote! {
                    pub async fn #raw_ident (
                        &self,
                        #(#input,)*
                    ) -> #_crate ::rpc::Result<#output, #_crate ::GetPubSub<#notif, #service>>
                    {
                        #[allow(unused_braces)]
                        self.0.process(
                            #req,
                            #duration,
                            ::core::result::Result::Ok(|__resp| match __resp {
                                #resp :: #ident (__ret) => #resp_map,
                                _ => ::core::option::Option::None,
                            }),
                        )
                        .await
                    }
                }
            } else {
                quote! {
                    pub fn #raw_ident (
                        &self,
                        #(#input,)*
                    ) -> #_crate ::rpc::Result<(), #_crate ::GetPubSub<#notif, #service>>
                    {
                        #[allow(unused_braces)]
                        self.0.process_send_only(#req)
                    }
                }
            }
        };

        // Request
        tokens.extend({
            let (indexes, fields): (TokenStream, TokenStream) = self
                .parse
                .handlers
                .iter()
                .enumerate()
                .map(|(i, (ident, sig))| {
                    let inputs = &sig.base.input;
                    (
                        quote!(#req :: #ident { .. } => #i,),
                        quote! { #ident { #(#inputs,)* }, },
                    )
                })
                .unzip();
            let dbg = req_dbg.then_some(quote!(#[derive(::core::fmt::Debug)]));
            let mut tokens = quote! {
                #dbg
                #[derive(::core::clone::Clone)]
                pub enum #req {
                    #fields
                }
                impl #_crate ::event::traits::Event<#notif> for #req {
                    type Service = #service;
                }
            };
            if !indexes.is_empty() {
                tokens.extend(quote! {
                    impl ::core::convert::From<&#req> for usize {
                        fn from(req: &#req) -> usize {
                            match req {
                                #indexes
                            }
                        }
                    }
                })
            }
            tokens
        });

        // Response
        tokens.extend({
            let (indexes, fields): (TokenStream, TokenStream) = self
                .parse
                .handlers
                .iter()
                .enumerate()
                .filter_map(|(i, (ident, sig))| {
                    let Some(out) = sig.base.output.as_ref() else {
                        return None;
                    };
                    Some((
                        quote!(#resp :: #ident (_) => #i,),
                        quote! { #ident (#out), },
                    ))
                })
                .unzip();
            let dbg = resp_dbg.then_some(quote!(#[derive(::core::fmt::Debug)]));
            let mut tokens = quote! {
                #dbg
                #[derive(::core::clone::Clone)]
                pub enum #resp {
                    #fields
                }
                impl #_crate ::event::traits::Event<#notif> for #resp {
                    type Service = #service;
                }
            };
            if !indexes.is_empty() {
                tokens.extend(quote! {
                    impl ::core::convert::From<&#resp> for usize {
                        fn from(resp: &#resp) -> usize {
                            match resp {
                                #indexes
                            }
                        }
                    }
                })
            }
            tokens
        });

        let mut out = TokenStream::default();

        // Impl RpcProvider
        out.extend({
            let err = err.as_ref().map(|err| quote!(type Error = #err;));
            quote! {
                impl #_crate ::rpc::traits::RpcProvider<#notif> for #service {
                    type Rpc = #_impl;
                    type Request = #req;
                    type Response = #resp;
                    #err
                    fn __new_rpc(__rpc: #_crate ::rpc::Rpc<Self::Impl>) -> Self::Rpc {
                        #_impl (__rpc)
                    }
                }
            }
        });

        // Rpc impl
        out.extend(quote! {
            #[allow(non_camel_case_types)]
            pub struct #_impl (
                #_crate ::rpc::Rpc<#_crate ::GetPubSub<#notif, #service>>,
            );
        });

        // Impl Rpc impl
        out.extend({
            let (commands, interfaces): (TokenStream, TokenStream) = self
                .parse
                .handlers
                .iter()
                .map(
                    |(
                        ident,
                        parse::Signature {
                            base:
                                sig @ parse::SigBase {
                                    raw_ident,
                                    duration,
                                    ..
                                },
                            interface:
                                parse::Interface {
                                    skip,
                                    alias: new_ident,
                                    commands: interface,
                                },
                            ..
                        },
                    )| {
                        let raw = new_ident.as_ref().unwrap_or(raw_ident);
                        let def = generate_fn(ident, raw, sig, None, None, duration.as_ref());
                        if interface.is_empty() {
                            return (def, quote!());
                        }
                        let interface_ident = Ident::new(
                            &format!("__{}_interface", raw_ident.to_string()),
                            raw_ident.span(),
                        );
                        let commands = interface
                            .iter()
                            .flat_map(
                                |parse::InterfaceSignature {
                                     base:
                                         sig @ parse::SigBase {
                                             raw_ident,
                                             duration: interface_duration,
                                             ..
                                         },
                                     block,
                                     response,
                                 }| {
                                    let r#fn = generate_fn(
                                        ident,
                                        raw_ident,
                                        sig,
                                        Some(block),
                                        response.as_ref(),
                                        interface_duration.as_ref().or(duration.as_ref()),
                                    );
                                    quote!(#r#fn)
                                },
                            )
                            .chain((!skip).then_some(def).into_token_stream())
                            .collect::<TokenStream>();
                        (
                            quote! {
                                pub fn #raw_ident <'r>(&'r self) -> #interface_ident <'r> {
                                    #interface_ident (&self.0)
                                }
                            },
                            quote! {
                                #[allow(non_camel_case_types)]
                                pub struct #interface_ident<'r> (
                                    &'r #_crate ::rpc::Rpc<#_crate ::GetPubSub<#notif, #service>>,
                                );
                                impl #interface_ident <'_> { #commands }
                            },
                        )
                    },
                )
                .unzip();
            quote! {
                impl #_impl { #commands }
                #interfaces
            }
        });

        tokens.extend(quote!(const _: () = { #out };));
    }
}
