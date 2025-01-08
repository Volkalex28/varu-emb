use enum_as_derive::EnumAs;
use proc_macro2::TokenStream;
use quote::ToTokens;
use std::cell::RefCell;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse, Result, Token};
use syn_derive::Parse;

#[derive(Parse, Clone, EnumAs)]
enum GenericParam {
    #[parse(peek = Token![=])]
    Output {
        _eq: Token![=],
        ty: syn::Type,
    },
    Param(syn::GenericParam),
}

struct Inner {
    generics: Vec<Punctuated<GenericParam, Token![,]>>,
    input: syn::ItemType,
}

pub struct CfgTypeAlias(RefCell<Inner>);
impl Parse for CfgTypeAlias {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let mut attrs = input.call(syn::Attribute::parse_outer)?;
        let input = input.parse::<syn::ItemType>()?;
        if !matches!(input.ty.as_ref(), syn::Type::Path(_)) {
            return Err(syn::Error::new(input.ty.span(), "Supported only for type aliases with path type"));
        }
        let generics = attrs
            .extract_if(|attr| attr.path().is_ident("bound"))
            .map(|attr| attr.parse_args_with(Punctuated::parse_separated_nonempty))
            .try_collect::<Vec<Punctuated<_, Token![,]>>>()?;

        Ok(Self(RefCell::new(Inner { generics, input })))
    }
}
impl ToTokens for CfgTypeAlias {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut this = self.0.borrow_mut();

        let generics = core::mem::take::<Vec<_>>(&mut this.generics).into_iter().flat_map(|generics| generics.into_iter());
        this.input.generics.params.extend(generics.clone().into_iter().filter_map(GenericParam::into_param));
        let syn::Type::Path(type_path) = this.input.ty.as_mut() else { unreachable!() };
        if let Some(segm) = type_path.path.segments.last_mut() {
            let args = generics.map(|g| match g {
                GenericParam::Param(syn::GenericParam::Lifetime(lifetime_param)) => {
                    syn::GenericArgument::Lifetime(lifetime_param.lifetime)
                }
                GenericParam::Param(syn::GenericParam::Type(type_param)) => {
                    let ident = &type_param.ident;
                    syn::GenericArgument::Type(syn::parse_quote! { #ident })
                }
                GenericParam::Param(syn::GenericParam::Const(const_param)) => {
                    let ident = &const_param.ident;
                    syn::GenericArgument::Const(syn::parse_quote! { #ident })
                }
                GenericParam::Output { ty, .. } => syn::GenericArgument::Type(syn::parse_quote! { #ty }),
            });
            match &mut segm.arguments {
                syn::PathArguments::AngleBracketed(arguments) => arguments.args.extend(args),
                arguments @ syn::PathArguments::None => {
                    *arguments = syn::PathArguments::AngleBracketed(syn::parse_quote! { < #(#args),* > })
                }
                _ => {}
            }
        }

        this.input.to_tokens(tokens);
    }
}
