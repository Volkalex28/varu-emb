use crate::{Error, Result};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned as _;
use syn::{parse, Token};
mod parsing;

#[derive(Debug)]
struct Spanned<T> {
    span: Span,
    data: T,
}

#[derive(Debug)]
enum PeripheralCrossError {
    Type(syn::Type),
    Traits(Vec<syn::TypeParamBound>),
}
impl ToTokens for PeripheralCrossError {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(quote! { Error });
        tokens.extend(match self {
            Self::Type(ty) => quote! { = #ty },
            Self::Traits(vec) => quote! { : #(#vec +)* },
        })
    }
}

#[derive(Debug)]
struct PeripheralCross {
    config: syn::Type,
    error: Option<PeripheralCrossError>,
}

#[derive(Debug)]
struct PeripheralItem {
    attrs: Vec<syn::Attribute>,
    cross: PeripheralCross,
    ident: syn::Ident,
    bounds: Punctuated<syn::TypeParamBound, Token![+]>,
}
impl PeripheralItem {
    fn bound(&self) -> Result<TokenStream> {
        let ident = &self.ident;
        let config = &self.cross.config;
        let error = self.cross.error.as_ref();

        let bound = quote! {
            bound( where Self: ::varuemb::cross::Peripheral< Self:: #ident, Config = #config, #error >)
        };

        self::bound(&self.attrs, bound)
    }
}
impl ToTokens for PeripheralItem {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { attrs, ident, bounds, .. } = self;

        tokens.extend(quote! {
            #(#attrs)*
            type #ident: #bounds;
        });
    }
}

#[derive(Debug)]
struct Module {
    attrs: Vec<syn::Attribute>,
    ident: syn::Ident,

    peripheral: Spanned<Vec<PeripheralItem>>,
    hw: Spanned<syn::FieldsNamed>,
    stmts: Vec<syn::Stmt>,
}
impl Module {
    fn bound(&self) -> Result<TokenStream> {
        let ident = &self.ident;
        let bound = quote! { bound( where P: #ident :: Peripheral ) };

        self::bound(&self.attrs, bound)
    }

    fn trait_bound(&self) -> Result<TokenStream> {
        let ident = &self.ident;
        let bound = quote! { bound( #ident :: Peripheral ) };

        self::bound(&self.attrs, bound)
    }

    fn wrap_type(ty: &mut syn::Type) -> Result<()> {
        match ty {
            syn::Type::Array(ty) => Self::wrap_type(ty.elem.as_mut())?,
            syn::Type::Ptr(ty) => Self::wrap_type(ty.elem.as_mut())?,
            syn::Type::Reference(ty) => Self::wrap_type(ty.elem.as_mut())?,
            syn::Type::Slice(ty) => Self::wrap_type(ty.elem.as_mut())?,
            syn::Type::Tuple(ty) => {
                for elem in &mut ty.elems {
                    Self::wrap_type(elem)?;
                }
            }
            syn::Type::Path(ty) if ty.qself.is_none() && ty.path.get_ident().is_some() => {
                let path = ty.path.get_ident().cloned().unwrap();
                ty.path = syn::parse_quote!(::varuemb::cross::Hw<P, P :: #path>);
            }
            _ => return Err(Error::new(ty.span(), "Unsupported type")),
        }
        Ok(())
    }

    fn field(field: &syn::Field) -> Result<TokenStream> {
        let attrs = &field.attrs;
        let ident = field.ident.as_ref();

        let mut ty = field.ty.clone();
        Self::wrap_type(&mut ty)?;

        Ok(quote! {
            #(#attrs)*
            pub #ident: #ty,
        })
    }
}
impl ToTokens for Module {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let stmts = &self.stmts;

        let mut content = quote! { #(#stmts)* };

        let peripheral = syn::Ident::new("Peripheral", self.peripheral.span);
        let bounds = match self.peripheral.data.iter().map(PeripheralItem::bound).try_collect::<Vec<_>>() {
            Ok(bounds) => bounds,
            Err(error) => {
                tokens.extend(error.into_compile_error());
                return;
            }
        };
        let types = self.peripheral.data.iter().map(ToTokens::to_token_stream);
        content.extend(quote! {
            #[::varuemb::utils::cfg_trait_bound]
            #(#bounds)*
            pub trait #peripheral {
                #(#types)*
            }
        });

        let hw = syn::Ident::new("Hw", self.hw.span);
        let fields = match self.hw.data.named.iter().map(Self::field).try_collect::<Vec<_>>() {
            Ok(fields) => fields,
            Err(error) => {
                tokens.extend(error.into_compile_error());
                return;
            }
        };
        content.extend(quote! {
            pub struct #hw <P: #peripheral> {
                #(#fields)*
            }
        });

        let attrs = &self.attrs;
        let ident = &self.ident;
        tokens.extend(quote! {
            #(#attrs)*
            pub mod #ident { #content }
        })
    }
}

#[derive(Debug)]
pub struct Hal {
    is_for: bool,
    path: syn::Path,
    _in: Option<syn::Path>,
    modules: Vec<Module>,
}
impl parse::Parse for Hal {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        input.parse::<parsing::Hal>()?.try_into()
    }
}
impl ToTokens for Hal {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = &self.path;
        let modules = &self.modules;

        let mut content = TokenStream::new();

        if !self.is_for {
            content.extend(quote! {
                pub struct #path <P = ()> (::core::marker::PhantomData<P>);
            });
        }

        let (modules_bound, modules_trait_bound): (Vec<_>, Vec<_>) = match modules
            .iter()
            .map(|module| {
                let bound = module.bound()?;
                let trait_bound = module.trait_bound()?;
                Result::Ok((bound, trait_bound))
            })
            .try_collect::<Vec<_>>()
        {
            Ok(bounds) => bounds.into_iter().unzip(),
            Err(error) => {
                content.extend(error.into_compile_error());
                return;
            }
        };

        content.extend({
            quote! {
                #[::varuemb::utils::cfg_trait_bound]
                #(#modules_trait_bound)*
                pub trait Platform = ::varuemb::cross::Platform;
            }
        });

        content.extend(quote! {
            #[::varuemb::utils::cfg_impl_block]
            #(#modules_bound)*
            impl<P> ::varuemb::cross::Interface<P> for #path <P> {
                type Hardware = Hardware<P>;
            }
        });

        for module in modules {
            module.to_tokens(&mut content);
        }

        let fields = self.modules.iter().map(|m| {
            let attrs = &m.attrs;
            let ident = &m.ident;
            quote! {
                #(#attrs)*
                pub #ident: #ident :: Hw<P>,
            }
        });
        content.extend(quote! {
            #[::varuemb::utils::cfg_derivable_item]
            #(#modules_bound)*
            pub struct Hardware <P> {
                #(#fields)*
            }
        });

        tokens.extend(if let Some(in_path) = &self._in {
            quote! {
                pub mod #in_path { #content }
            }
        } else {
            content
        })
    }
}

fn bound(attrs: &[syn::Attribute], bound: TokenStream) -> Result<TokenStream> {
    if attrs.is_empty() {
        return Ok(quote! { #[#bound] });
    }

    let cfgs = {
        const MESSAGE: &'static str = "Supported only cfg attribute";

        let attrs = attrs.iter();
        let mut attrs = attrs.map(|a| {
            if a.path().is_ident("cfg") {
                a.parse_args::<TokenStream>()
            } else {
                Err(Error::new(a.span(), MESSAGE))
            }
        });
        attrs.try_collect::<Vec<_>>()?
    };

    Ok(quote! { #[cfg_attr(any(#(#cfgs,)*), #bound)] })
}
