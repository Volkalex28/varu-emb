use crate::{Error, Parenthesized, Result};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::{error, result};
use syn::spanned::Spanned;
use syn::{parse, token, ItemStruct};

#[path = "attributes.rs"]
mod attrs;

use self::attrs::Members;

#[derive(Debug, derive_builder::Builder)]
struct I2c {
    #[builder(setter(custom, strip_option))]
    #[builder(field(ty = "Option<syn::Type>", build = "self.error.clone()"))]
    error: Option<syn::Type>,

    #[builder(default, setter(custom))]
    asynch: bool,

    #[builder(default, setter(custom))]
    blocking: bool,
}
impl I2cBuilder {
    fn set_properties(&mut self, property: impl IntoIterator<Item = attrs::Property>) -> Result<()> {
        for property in property {
            match property {
                attrs::Property::Error { ty, token, .. } => set(&mut self.error, ty, token)?,
                attrs::Property::Async { token, .. } => set(&mut self.asynch, true, token)?,
                attrs::Property::Blocking { token, .. } => set(&mut self.blocking, true, token)?,
            }
        }
        Ok(())
    }
}
impl I2c {
    fn to_tokens(&self, input: &syn::Ident, ident: &syn::Member, ty: &syn::Type, tokens: &mut TokenStream) {
        let error = self.error.as_ref();
        let transform = error.map(|err| quote! { [Self::Error |__e| { #err :: from(__e) } -> #err] });
        let bounds = error.map(|err| {
            quote! {
                where #err: ::core::convert::From<<#ty as ::varuemb::cross::i2c::ErrorType>::Error>
            }
        });

        let asynch = self.asynch.then(|| quote! { + ::varuemb::cross::i2c::asynch::I2c });
        let blocking = self.blocking.then(|| quote! { + ::varuemb::cross::i2c::I2c });

        tokens.extend(quote! {
            ::forward_traits::forward_traits! {
                for #input . #ident #transform #bounds
                impl ::varuemb::cross::i2c::ErrorType #blocking #asynch
            }
        })
    }
}

#[derive(Debug, derive_builder::Builder)]
struct IoModes {
    #[builder(default, setter(custom))]
    read: bool,
    #[builder(default, setter(custom))]
    write: bool,
}

#[derive(Debug, derive_builder::Builder)]
#[builder(build_fn(error = "Box<dyn error::Error>"))]
struct Io {
    #[builder(setter(custom, strip_option))]
    #[builder(field(ty = "Option<syn::Type>", build = "self.error.clone()"))]
    error: Option<syn::Type>,

    #[builder(setter(custom))]
    #[builder(field(ty = "IoModesBuilder", build = "self.asynch.build()?"))]
    asynch: IoModes,

    #[builder(setter(custom))]
    #[builder(field(ty = "IoModesBuilder", build = "self.blocking.build()?"))]
    blocking: IoModes,
}
impl IoBuilder {
    fn set_properties(&mut self, property: impl IntoIterator<Item = attrs::Property<attrs::io::Part>>) -> Result<()> {
        for property in property {
            match property {
                attrs::Property::Error { ty, token, .. } => set(&mut self.error, ty, token)?,
                attrs::Property::Async { data: Some(data), .. } => Self::set_mode(&mut self.asynch, data)?,
                attrs::Property::Blocking { data: Some(data), .. } => Self::set_mode(&mut self.blocking, data)?,
                attrs::Property::Async { data: None, .. } | attrs::Property::Blocking { data: None, .. } => {}
            }
        }
        Ok(())
    }

    fn set_mode(mode: &mut IoModesBuilder, data: Parenthesized<attrs::io::Part>) -> Result<()> {
        data.data.into_iter().try_for_each(|part| match part {
            attrs::io::Part::Read(token) => set(&mut mode.read, true, token),
            attrs::io::Part::Write(token) => set(&mut mode.write, true, token),
        })
    }
}
impl Io {
    fn to_tokens(&self, input: &syn::Ident, ident: &syn::Member, ty: &syn::Type, tokens: &mut TokenStream) {
        let asynch_read = self.asynch.read.then(|| quote! { + ::varuemb::cross::io::asynch::Read });
        let asynch_write = self.asynch.write.then(|| quote! { + ::varuemb::cross::io::asynch::Write });

        let blocking_read = self.blocking.read.then(|| quote! { + ::varuemb::cross::io::Read });
        let blocking_write = self.blocking.write.then(|| quote! { + ::varuemb::cross::io::Write });

        let error = self.error.as_ref();
        let transform = error.map(|err| {
            let blocking = (blocking_read.is_some() || blocking_write.is_some()).then(|| {
                quote! {
                    , ::varuemb::cross::io::WriteFmtError<Self::Error> |err| {
                        ::varuemb::cross::io::__private::write_fmt_error(err)
                    } -> ::varuemb::cross::io::WriteFmtError<#err>
                }
            });
            quote! {
                [
                    Self::Error |__e| { #err :: from(__e) } -> #err,
                    ::varuemb::cross::io::ReadExactError<Self::Error> |err| {
                        ::varuemb::cross::io::__private::read_exact_error(err)
                    } -> ::varuemb::cross::io::ReadExactError<#err>
                    #blocking
                ]
            }
        });
        let bounds = error.map(|err| {
            quote! {
                where #err: ::core::convert::From<<#ty as ::varuemb::cross::io::ErrorType>::Error>
            }
        });

        tokens.extend(quote! {
            ::forward_traits::forward_traits! {
                for #input . #ident #transform #bounds
                impl ::varuemb::cross::io::ErrorType #blocking_read #blocking_write #asynch_read #asynch_write
            }
        })
    }
}

#[derive(Debug, derive_builder::Builder)]
struct SpiTypes {
    #[builder(default, setter(custom))]
    bus: bool,
    #[builder(default, setter(custom))]
    device: bool,
}

#[derive(Debug, derive_builder::Builder)]
#[builder(build_fn(error = "Box<dyn error::Error>"))]
struct Spi {
    #[builder(setter(custom, strip_option))]
    #[builder(field(ty = "Option<syn::Type>", build = "self.error.clone()"))]
    error: Option<syn::Type>,

    #[builder(setter(custom))]
    #[builder(field(ty = "SpiTypesBuilder", build = "self.asynch.build()?"))]
    asynch: SpiTypes,

    #[builder(setter(custom))]
    #[builder(field(ty = "SpiTypesBuilder", build = "self.blocking.build()?"))]
    blocking: SpiTypes,
}
impl SpiBuilder {
    fn set_properties(&mut self, property: impl IntoIterator<Item = attrs::Property<attrs::spi::Type>>) -> Result<()> {
        for property in property {
            match property {
                attrs::Property::Error { ty, token, .. } => set(&mut self.error, ty, token)?,
                attrs::Property::Async { data: Some(data), .. } => Self::set_type(&mut self.asynch, data)?,
                attrs::Property::Blocking { data: Some(data), .. } => Self::set_type(&mut self.blocking, data)?,
                attrs::Property::Async { data: None, .. } | attrs::Property::Blocking { data: None, .. } => {}
            }
        }
        Ok(())
    }

    fn set_type(ty: &mut SpiTypesBuilder, data: Parenthesized<attrs::spi::Type>) -> Result<()> {
        data.data.into_iter().try_for_each(|part| match part {
            attrs::spi::Type::Bus(token) => set(&mut ty.bus, true, token),
            attrs::spi::Type::Device(token) => set(&mut ty.device, true, token),
        })
    }
}
impl Spi {
    fn to_tokens(&self, input: &syn::Ident, ident: &syn::Member, ty: &syn::Type, tokens: &mut TokenStream) {
        let error = self.error.as_ref();
        let transform = error.map(|err| quote! { [Self::Error |__e| { #err :: from(__e) } -> #err] });
        let bounds = error.map(|err| {
            quote! {
                where #err: ::core::convert::From<<#ty as ::varuemb::cross::spi::ErrorType>::Error>
            }
        });

        let asynch_bus = self.asynch.bus.then(|| quote! { + ::varuemb::cross::spi::bus::asynch::SpiBus });
        let asynch_device = self.asynch.device.then(|| quote! { + ::varuemb::cross::spi::device::asynch::SpiDevice });

        let blocking_bus = self.blocking.bus.then(|| quote! { + ::varuemb::cross::spi::bus::SpiBus });
        let blocking_device = self.blocking.device.then(|| quote! { + ::varuemb::cross::spi::device::SpiDevice });

        tokens.extend(quote! {
            ::forward_traits::forward_traits! {
                for #input . #ident #transform #bounds
                impl ::varuemb::cross::spi::ErrorType #blocking_bus #blocking_device #asynch_bus #asynch_device
            }
        })
    }
}

#[derive(Default)]
struct Builders {
    i2c: Option<I2cBuilder>,
    spi: Option<SpiBuilder>,
    io: Option<IoBuilder>,
}

#[derive(Debug)]
enum Interface {
    I2c(I2c),
    Io(Io),
    Spi(Spi),
}

#[derive(Debug)]
struct Member {
    ident: syn::Member,
    interfaces: Vec<Interface>,
}
impl Member {
    fn new(input: Members) -> Result<Vec<Member>> {
        let mut sorted = HashMap::new();
        for member in input {
            sorted.entry(member.ident).or_insert_with(Vec::new).push(member.interface);
        }

        let mut members = HashMap::new();
        for (member, interfaces) in sorted {
            let builders = members.entry(member).or_insert_with(Builders::default);
            for interface in interfaces {
                use attrs::Interface::*;
                match interface {
                    I2c { props, .. } => builders.i2c.get_or_insert_default().set_properties(props.data),
                    Io { props, .. } => builders.io.get_or_insert_default().set_properties(props.data),
                    Spi { props, .. } => builders.spi.get_or_insert_default().set_properties(props.data),
                }?
            }
        }

        let mut members = members.into_iter().map(|(member, builders)| -> result::Result<_, Box<dyn error::Error>> {
            let mut member = Member { ident: member, interfaces: Vec::new() };

            if let Some(i2c) = builders.i2c {
                member.interfaces.push(Interface::I2c(i2c.build()?));
            }
            if let Some(io) = builders.io {
                member.interfaces.push(Interface::Io(io.build()?));
            }
            if let Some(spi) = builders.spi {
                member.interfaces.push(Interface::Spi(spi.build()?));
            }

            Ok(member)
        });

        members.try_collect().map_err(|err| Error::new(Span::call_site(), format!("{err}")))
    }
}

pub struct Forward {
    input: ItemStruct,
    members: Vec<Member>,
}

impl parse::Parse for Forward {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let input = input.parse::<ItemStruct>()?;

        let tokens = crate::extract_attrs(&input.attrs, &["cross", "forward"])?;
        let members = parse::Parser::parse2(Members::parse_terminated, tokens)?;

        for member in members.iter() {
            let contains = match &member.ident {
                syn::Member::Named(ident) => {
                    input.fields.iter().any(|field| field.ident.as_ref().is_some_and(|i| i == ident))
                }
                syn::Member::Unnamed(index) => input.fields.len() > index.index as usize,
            };
            if !contains {
                let message = format!(
                    "Field {ident} is not present in {input}",
                    ident = format_ident!("{}", &member.ident),
                    input = input.ident
                );
                return Err(Error::new(member.ident.span(), message));
            }
        }

        Member::new(members).map(|members| Self { input, members })
    }
}
impl quote::ToTokens for Forward {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for member in &self.members {
            let Some((_, field)) = self.input.fields.iter().enumerate().find(|(i, f)| match &member.ident {
                syn::Member::Named(ident) => f.ident.as_ref().is_some_and(|i| i == ident),
                syn::Member::Unnamed(index) => index.index as usize == *i,
            }) else {
                unreachable!()
            };

            for interface in member.interfaces.iter() {
                match interface {
                    Interface::I2c(i2c) => i2c.to_tokens(&self.input.ident, &member.ident, &field.ty, tokens),
                    Interface::Io(io) => io.to_tokens(&self.input.ident, &member.ident, &field.ty, tokens),
                    Interface::Spi(spi) => spi.to_tokens(&self.input.ident, &member.ident, &field.ty, tokens),
                }
            }
        }
    }
}

mod tokens {
    syn::custom_keyword!(i2c);
    syn::custom_keyword!(io);
    syn::custom_keyword!(spi);

    syn::custom_keyword!(error);
    syn::custom_keyword!(blocking);
}

fn set<D, T: token::Token + Spanned>(data: &mut Option<D>, value: D, token: T) -> Result<()> {
    if data.replace(value).is_some() {
        return Err(syn::Error::new(token.span(), format!("Duplicate {name}", name = T::display())));
    }
    Ok(())
}
