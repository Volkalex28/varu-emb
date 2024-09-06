use crate::{Error, Result};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::sync::LazyLock;
use syn::{
    parse::{self},
    spanned::Spanned,
    DeriveInput, Token,
};

#[cfg(not(feature = "testing"))]
const PATH: LazyLock<TokenStream> = LazyLock::new(|| quote! { ::varuemb::devices::register:: });
#[cfg(feature = "testing")]
const PATH: LazyLock<TokenStream> = LazyLock::new(|| quote! { crate::register:: });

enum Address {
    Skip,
    Offset(syn::Expr),
}
impl Default for Address {
    fn default() -> Self {
        Self::Offset(syn::parse_quote!(0))
    }
}
impl parse::Parse for Address {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(tokens::skip) {
            input.parse::<tokens::skip>().map(|_| Self::Skip)
        } else if lookahead.peek(tokens::offset) {
            input.parse::<tokens::offset>()?;
            input.parse::<Token![:]>()?;
            input.parse().map(Self::Offset)
        } else {
            Err(lookahead.error())
        }
    }
}
impl ToTokens for Address {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let addr = match self {
            Self::Skip => quote! { None },
            Self::Offset(offset) => quote! { Some(#offset) },
        };
        tokens.extend(quote! { ::core::option::Option:: #addr })
    }
}

enum Config {
    Skip,
    Data { address: Address, timeout: syn::Expr, size: syn::Expr },
}
impl Default for Config {
    fn default() -> Self {
        let path = &*PATH;
        Self::Data {
            address: Address::default(),
            timeout: syn::parse_quote!(#path Duration::from_millis(0)),
            size: syn::parse_quote!(::core::mem::size_of::<Self>()),
        }
    }
}
impl Config {
    fn set_base_address(&mut self, address: &syn::Expr) {
        if let Self::Data { address: Address::Offset(offset), .. } = self {
            *offset = syn::parse_quote!((#address) + (#offset));
        }
    }
}
impl parse::Parse for Config {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        if input.peek(tokens::skip) {
            input.parse::<tokens::skip>()?;
            return Ok(Self::Skip);
        }

        let mut parse_address = Option::None;
        let mut parse_timeout = Option::None;
        let mut parse_size = Option::None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if parse_paren(&mut parse_address, &lookahead, input, tokens::address)? {
            } else if parse(&mut parse_timeout, &lookahead, input, tokens::timeout)? {
            } else if parse(&mut parse_size, &lookahead, input, tokens::size)? {
            } else if lookahead.peek(tokens::skip) {
                return Err(Error::new(input.span(), "Attribute \"skip\" should be first"));
            } else {
                return Err(lookahead.error());
            }
        }
        let Self::Data { address, timeout, size } = Self::default() else { unreachable!() };
        Ok(Self::Data {
            address: parse_address.unwrap_or(address),
            timeout: parse_timeout.unwrap_or(timeout),
            size: parse_size.unwrap_or(size),
        })
    }
}

#[derive(Default)]
enum Order {
    #[default]
    Msb,
    Lsb,
}
impl parse::Parse for Order {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(tokens::msb) {
            input.parse::<tokens::msb>().map(|_| Self::Msb)
        } else if lookahead.peek(tokens::lsb) {
            input.parse::<tokens::lsb>().map(|_| Self::Lsb)
        } else {
            Err(lookahead.error())
        }
    }
}
impl ToTokens for Order {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let order = match self {
            Self::Msb => quote! {Msb},
            Self::Lsb => quote! {Lsb},
        };
        let path = &*PATH;
        tokens.extend(quote! { #path Order:: #order })
    }
}

pub struct Register {
    count: syn::Expr,
    order: Order,
    read: Config,
    write: Config,
    input: DeriveInput,
}
impl Register {
    fn parse_attributes(input: DeriveInput, attrs: parse::ParseStream) -> Result<Self> {
        let mut address = Option::None;
        let mut count = Option::None;
        let mut order = Option::None;
        let mut read = Option::None;
        let mut write = Option::None;

        while !attrs.is_empty() {
            let lookahead = attrs.lookahead1();
            if parse(&mut address, &lookahead, attrs, tokens::address)? {
            } else if parse(&mut order, &lookahead, attrs, tokens::order)? {
            } else if parse(&mut count, &lookahead, attrs, tokens::count)? {
            } else if parse_paren(&mut read, &lookahead, attrs, tokens::read)? {
            } else if parse_paren(&mut write, &lookahead, attrs, tokens::write)? {
            } else {
                return Err(lookahead.error());
            }
        }

        let address = address.ok_or(Error::new(input.span(), "register address must be specified"))?;
        let mut this = Self {
            count: count.unwrap_or_else(|| syn::parse_quote!(1)),
            order: order.unwrap_or_default(),
            read: read.unwrap_or_default(),
            write: write.unwrap_or_default(),
            input,
        };

        this.read.set_base_address(&address);
        this.write.set_base_address(&address);

        Ok(this)
    }

    fn make_config(config: &Config) -> Option<TokenStream> {
        let Config::Data { address, timeout, size } = config else {
            return None;
        };

        let path = &*PATH;
        Some(quote! {
            const ADDRESS: ::core::option::Option<::core::primitive::u8> = #address;
            const SIZE: ::core::primitive::usize = { #size };
            const TIMEOUT: #path Duration = { #timeout };
        })
    }
}

impl parse::Parse for Register {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let input = input.parse::<DeriveInput>()?;
        let span = input.span();

        match &input.data {
            syn::Data::Struct(_) => Ok(()),
            syn::Data::Enum(_) => Err(Error::new(span, "Enum is not supported for Register")),
            syn::Data::Union(_) => Err(Error::new(span, "Union is not supported for Register")),
        }?;
        let attrs = input
            .attrs
            .iter()
            .filter_map(|a| {
                if !a.path().is_ident("register") {
                    return None;
                }
                Some(a.parse_args::<TokenStream>())
            })
            .try_collect::<Vec<_>>()?;

        if attrs.is_empty() {
            return Err(Error::new(span, "No attribute \"register\" found"));
        }

        let attrs = attrs.into_iter().flatten().collect::<TokenStream>();
        parse::Parser::parse2(move |attrs: parse::ParseStream| Self::parse_attributes(input, attrs), attrs)
    }
}

impl ToTokens for Register {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = &*PATH;
        let count = &self.count;
        let order = &self.order;

        let ident = &self.input.ident;
        let (r#impl, params, r#where) = self.input.generics.split_for_impl();

        tokens.extend(quote! {
            impl #r#impl #path Instance for #ident #params #r#where {
                const ORDER: #path Order = #order;
                const COUNT: ::core::primitive::usize = { #count };
            }
        });

        if let Some(read) = Self::make_config(&self.read) {
            tokens.extend(quote! {
                impl #r#impl #path Config<false> for #ident #params #r#where {
                    #read
                }
            });
        }

        if let Some(write) = Self::make_config(&self.write) {
            tokens.extend(quote! {
                impl #r#impl #path Config<true> for #ident #params #r#where {
                    #write
                }
            })
        }
    }
}

fn parse<T: parse::Peek<Token: parse::Parse + Spanned>, R: parse::Parse>(
    ret: &mut Option<R>,
    lookaheed: &parse::Lookahead1,
    input: &parse::ParseBuffer,
    token: T,
) -> syn::Result<bool> {
    if lookaheed.peek(token) {
        let token = input.parse::<T::Token>()?;
        input.parse::<syn::Token![:]>()?;

        if ret.is_some() {
            return Err(Error::new(token.span(), "Duplicate attribute"));
        }
        *ret = Some(input.parse()?);

        if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

fn parse_paren<T: parse::Peek<Token: parse::Parse + Spanned>, R: parse::Parse>(
    ret: &mut Option<R>,
    lookaheed: &parse::Lookahead1,
    input: &parse::ParseBuffer,
    token: T,
) -> syn::Result<bool> {
    if lookaheed.peek(token) {
        let token = input.parse::<T::Token>()?;

        let content;
        syn::parenthesized!(content in input);

        if ret.is_some() {
            return Err(Error::new(token.span(), "Duplicate attribute"));
        }
        *ret = Some(content.parse()?);

        if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

mod tokens {
    syn::custom_keyword!(count);

    syn::custom_keyword!(address);
    syn::custom_keyword!(offset);
    syn::custom_keyword!(skip);

    syn::custom_keyword!(order);
    syn::custom_keyword!(msb);
    syn::custom_keyword!(lsb);

    syn::custom_keyword!(read);
    syn::custom_keyword!(write);

    syn::custom_keyword!(timeout);
    syn::custom_keyword!(size);
}
