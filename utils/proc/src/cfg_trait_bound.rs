use proc_macro2::TokenStream;
use quote::ToTokens;
use std::cell::RefCell;
use syn::parse::discouraged::Speculative;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{parse, Result, Token};
use syn_derive::{Parse, ToTokens};

#[derive(ToTokens)]
pub enum TraitType {
    Trait(syn::ItemTrait),
    Alias(syn::ItemTraitAlias),
}
impl TraitType {
    fn attrs_mut(&mut self) -> &mut Vec<syn::Attribute> {
        match self {
            TraitType::Trait(item_trait) => item_trait.attrs.as_mut(),
            TraitType::Alias(item_trait_alias) => item_trait_alias.attrs.as_mut(),
        }
    }
}
impl Parse for TraitType {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let fork = input.fork();
        match fork.parse::<syn::ItemTraitAlias>() {
            Ok(alias) => {
                input.advance_to(&fork);
                Ok(Self::Alias(alias))
            }
            Err(_) => input.parse().map(Self::Trait),
        }
    }
}

#[derive(Parse)]
enum GenericAttribute {
    #[parse(peek = Token![where])]
    WhereClause(syn::WhereClause),
    Param(syn::TypeParamBound),
}
impl GenericAttribute {
    fn split(self) -> (Option<syn::WhereClause>, Option<syn::TypeParamBound>) {
        match self {
            GenericAttribute::WhereClause(where_clause) => (Some(where_clause), None),
            GenericAttribute::Param(param) => (None, Some(param)),
        }
    }
}

struct Inner {
    bounds: Vec<Punctuated<GenericAttribute, Token![|]>>,
    input: TraitType,
}

pub struct CfgTraitBound(RefCell<Inner>);
impl Parse for CfgTraitBound {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let mut input = input.parse::<TraitType>()?;
        let bounds = input
            .attrs_mut()
            .extract_if(|attr| attr.path().is_ident("bound"))
            .map(|attr| attr.parse_args_with(Punctuated::parse_separated_nonempty))
            .try_collect::<Vec<Punctuated<GenericAttribute, Token![|]>>>()?;

        Ok(Self(RefCell::new(Inner { bounds, input })))
    }
}
impl ToTokens for CfgTraitBound {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut this = self.0.borrow_mut();

        let (where_clause, param): (Vec<_>, Vec<_>) = core::mem::take::<Vec<_>>(&mut this.bounds)
            .into_iter()
            .flat_map(|bounds| bounds.into_iter())
            .map(|g| g.split())
            .unzip();

        match &mut this.input {
            TraitType::Trait(item_trait) => &mut item_trait.supertraits,
            TraitType::Alias(item_trait_alias) => &mut item_trait_alias.bounds,
        }
        .extend(param.into_iter().filter_map(|b| b));

        let generics = match &mut this.input {
            TraitType::Trait(item_trait) => &mut item_trait.generics,
            TraitType::Alias(item_trait_alias) => &mut item_trait_alias.generics,
        };

        let where_clause = where_clause.into_iter().filter_map(|b| b).flat_map(|b| b.predicates.into_iter());
        match &mut generics.where_clause {
            Some(generics) => generics.predicates.extend(where_clause),
            None => generics.where_clause = Some(syn::parse_quote! { where #(#where_clause,)* }),
        }

        this.input.to_tokens(tokens);
    }
}
