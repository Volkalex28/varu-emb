use crate::Result;
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{parse, Token};
use syn_derive::{Parse, ToTokens};

#[derive(Debug, Parse, ToTokens)]
enum Entry {
    #[parse(peek = Token![|])]
    Closure(syn::ExprClosure),
    Path(syn::Path),
}

#[derive(Parse)]
enum Attribute {
    #[parse(peek = tokens::alias)]
    Alias(crate::ValueAttribute<tokens::alias, syn::Ident>),
    #[parse(peek = tokens::infinity)]
    Infinity(crate::ValueAttribute<tokens::infinity, syn::LitBool>),
    #[parse(peek = tokens::entry)]
    Entry(crate::ValueAttribute<tokens::entry, Entry>),
}

#[derive(Debug)]
struct Attributes {
    alias: Option<syn::Ident>,
    entry: Option<Entry>,
    infinity: syn::LitBool,
}
impl Default for Attributes {
    fn default() -> Self {
        Self { alias: None, entry: None, infinity: syn::parse_quote!(true) }
    }
}
impl From<Vec<Attribute>> for Attributes {
    fn from(attrs: Vec<Attribute>) -> Self {
        let mut this = Self::default();

        for attr in attrs {
            match attr {
                Attribute::Alias(value) => this.alias = value.value.into(),
                Attribute::Infinity(value) => this.infinity = value.value.into(),
                Attribute::Entry(value) => this.entry = value.value.into(),
            }
        }

        this
    }
}

#[derive(Debug)]
pub struct Task {
    input: syn::DeriveInput,
    attributes: Attributes,
}
impl parse::Parse for Task {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let mut input = input.parse::<syn::DeriveInput>()?;

        let mut tokens = Vec::<Attribute>::new();
        for attr in input.attrs.drain(..).filter(|a| a.path().is_ident("varuemb_executor")) {
            use crate::ParenAttribute;

            #[derive(Parse)]
            struct ParseAttributes {
                #[parse(Punctuated::parse_terminated)]
                attributes: Punctuated<Attribute, Token![,]>,
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("task") {
                    let task = ParenAttribute::<syn::parse::Nothing, ParseAttributes>::parse(meta.input)?.content;
                    tokens.extend(task.attributes);
                }
                Ok(())
            })?;
        }

        Ok(Self { input, attributes: Attributes::from(tokens) })
    }
}
impl quote::ToTokens for Task {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.input.ident;
        let alias = self.attributes.alias.as_ref().unwrap_or(&ident);
        let (gen, ty, wh) = self.input.generics.split_for_impl();

        let func = syn::parse_quote!(Self::entry);
        let func = self.attributes.entry.as_ref().unwrap_or(&func);

        let name = alias.to_string();

        let span = proc_macro2::Span::mixed_site();
        let ident_lower = alias.to_string().to_snake_case();
        let log = syn::Ident::new(&(ident_lower.clone() + "_log"), span);
        let error = syn::Ident::new(&(ident_lower.clone() + "_error"), span);
        let warn = syn::Ident::new(&(ident_lower.clone() + "_warn"), span);
        let info = syn::Ident::new(&(ident_lower.clone() + "_info"), span);
        let debug = syn::Ident::new(&(ident_lower + "_debug"), span);

        let body = (!self.attributes.infinity.value())
            .then(|| {
                quote!(if let Err(err) = res {
                    ::log::error!(target: "Executor", "{} task aborted with error: {:?}", #name, err);
                } else {
                    ::log::warn!(target: "Executor", "{} task completed", #name);
                })
            })
            .unwrap_or(quote! {
                panic!("Infinity task {} completed with: {:?}", #name, res)
            });

        tokens.extend(quote!(
            #[allow(unused)]
            use ::varuemb::executor::TaskName as _;

            const _: () = {
            #[allow(non_camel_case_types)]
            pub trait _varuemb_internal {
                type Fut: ::core::future::Future<Output = ::core::result::Result<(), Self::Error>> + 'static;
                type Error: ::core::fmt::Debug + 'static;
                fn __entry(self) -> Self::Fut;
            }
            impl #gen _varuemb_internal for #ident #ty #wh {
                type Fut = impl ::core::future::Future<Output = ::core::result::Result<(), Self::Error>> + 'static;
                type Error = impl ::core::fmt::Debug + 'static;
                #[inline]
                fn __entry(self) -> Self::Fut {
                    (#func)(self)
                }
            }
            impl #gen ::varuemb::executor::Task for #ident #ty #wh {
                type Fut = <#ident #ty as _varuemb_internal>::Fut;

                fn __process(self) -> Self::Fut { <#ident #ty as _varuemb_internal>::__entry(self) }

                fn __finish(res: <Self::Fut as ::core::future::Future>::Output) { #body }
            }
            impl #gen ::varuemb::executor::TaskName for #ident #ty #wh
            {
                const NAME: &'static str = #name;

                #[inline]
                fn name() -> &'static str {
                    #name
                }
            } };


            pub mod log_self {
                #[allow(unused)]
                macro_rules! #log {
                    ($level:expr, $($tokens:tt)+) => {
                        ::log::log!(target: #name, $level, $($tokens)+)
                    };
                }

                #[allow(unused)]
                macro_rules! #error {
                    ($($tokens:tt)+) => {
                        ::log::error!(target: #name, $($tokens)+)
                    };
                }

                #[allow(unused)]
                macro_rules! #warn {
                    ($($tokens:tt)+) => {
                        ::log::warn!(target: #name, $($tokens)+)
                    };
                }

                #[allow(unused)]
                macro_rules! #info {
                    ($($tokens:tt)+) => {
                        ::log::info!(target: #name, $($tokens)+)
                    };
                }

                #[allow(unused)]
                macro_rules! #debug {
                    ($($tokens:tt)+) => {
                        ::log::debug!(target: #name, $($tokens)+)
                    };
                }

                #[allow(unused_imports)]
                pub(super) use #log as log;
                #[allow(unused_imports)]
                pub(super) use #error as error;
                #[allow(unused_imports)]
                pub(super) use #warn as warn;
                #[allow(unused_imports)]
                pub(super) use #info as info;
                #[allow(unused_imports)]
                pub(super) use #debug as debug;
            }
        ))
    }
}
mod tokens {
    syn::custom_keyword!(task);

    syn::custom_keyword!(alias);
    syn::custom_keyword!(generics);
    syn::custom_keyword!(infinity);
    syn::custom_keyword!(entry);
}
