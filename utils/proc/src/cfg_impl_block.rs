use proc_macro2::TokenStream;
use quote::ToTokens;
use std::cell::RefCell;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse, Result, Token};
use syn_derive::Parse;

#[derive(Parse)]
enum GenericAttribute {
    #[parse(peek = Token![where])]
    WhereClause(syn::WhereClause),
    Param(syn::GenericParam),
}
impl GenericAttribute {
    fn split(self) -> (Option<syn::WhereClause>, Option<syn::GenericParam>) {
        match self {
            GenericAttribute::WhereClause(where_clause) => (Some(where_clause), None),
            GenericAttribute::Param(param) => (None, Some(param)),
        }
    }
}

struct Inner {
    generics: Vec<Punctuated<GenericAttribute, Token![|]>>,
    input: syn::ItemImpl,
}

pub struct CfgImplBlock(RefCell<Inner>);
impl Parse for CfgImplBlock {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let mut attrs = input.call(syn::Attribute::parse_outer)?;
        let input = input.parse::<syn::ItemImpl>()?;
        if !matches!(input.self_ty.as_ref(), syn::Type::Path(_)) {
            return Err(syn::Error::new(input.self_ty.span(), "Supported only for type aliases with path type"));
        }
        let generics = attrs
            .extract_if(|attr| attr.path().is_ident("bound"))
            .map(|attr| attr.parse_args_with(Punctuated::parse_separated_nonempty))
            .try_collect::<Vec<Punctuated<_, Token![|]>>>()?;

        // panic!("{input:#?}");

        Ok(Self(RefCell::new(Inner { generics, input })))
    }
}
impl ToTokens for CfgImplBlock {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut this = self.0.borrow_mut();

        let (where_clauses, generics): (Vec<_>, Vec<_>) = core::mem::take::<Vec<_>>(&mut this.generics)
            .into_iter()
            .flat_map(|generics| generics.into_iter().map(|g| g.split()))
            .unzip();
        this.input.generics.params.extend(generics.clone().into_iter().filter_map(|g| g));
        for where_clause in where_clauses {
            if let Some(where_clause) = where_clause {
                if let Some(generics) = this.input.generics.where_clause.as_mut() {
                    generics.predicates.extend(where_clause.predicates)
                } else {
                    this.input.generics.where_clause = Some(where_clause)
                }
            }
        }

        let syn::Type::Path(type_path) = this.input.self_ty.as_mut() else { unreachable!() };
        if let Some(segm) = type_path.path.segments.last_mut() {
            let args = generics.into_iter().filter_map(|g| g).map(|g| match g {
                syn::GenericParam::Lifetime(lifetime_param) => syn::GenericArgument::Lifetime(lifetime_param.lifetime),
                syn::GenericParam::Type(type_param) => {
                    let ident = &type_param.ident;
                    syn::GenericArgument::Type(syn::parse_quote! { #ident })
                }
                syn::GenericParam::Const(const_param) => {
                    let ident = &const_param.ident;
                    syn::GenericArgument::Const(syn::parse_quote! { #ident })
                }
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
