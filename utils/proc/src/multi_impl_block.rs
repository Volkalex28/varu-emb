use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt};
use syn::parse::Parse;
use syn::spanned::Spanned;
use syn::{parse, parse_quote, AttrStyle, Block, Error, Generics, Path, Result, Token};
use syn_derive::Parse;

#[derive(Parse)]
struct ItemImpl {
    #[parse(syn::Attribute::parse_outer)]
    pub attrs: Vec<syn::Attribute>,
    pub defaultness: Option<Token![default]>,
    pub unsafety: Option<Token![unsafe]>,
    pub impl_token: Token![impl],
    #[parse(Self::parse_constantly)]
    pub constantly: Option<(Option<Token![?]>, Token![const])>,
    pub generics: Generics,

    /// Trait this impl implements.
    #[parse(|_| Ok(None))]
    pub trait_: Option<(Option<Token![!]>, Path, Token![for])>,
    /// The Self type of the impl.
    pub self_ty: Box<syn::Type>,

    #[syn(braced)]
    pub _brace_token: syn::token::Brace,

    #[syn(in = _brace_token)]
    #[parse(Self::parse_items)]
    pub items: Vec<syn::ImplItem>,
}
impl ItemImpl {
    fn parse_constantly(input: &parse::ParseBuffer) -> Result<Option<(Option<Token![?]>, Token![const])>> {
        let is_const_impl = input.peek(Token![const]) || input.peek(Token![?]) && input.peek2(Token![const]);
        is_const_impl
            .then(|| {
                let const_0 = input.parse::<Option<Token![?]>>()?;
                let const_1 = input.parse::<Token![const]>()?;
                Ok((const_0, const_1))
            })
            .transpose()
    }

    fn parse_items(input: &parse::ParseBuffer) -> Result<Vec<syn::ImplItem>> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
        }
        Ok(items)
    }
}

pub struct MultiImplBlock {
    stmts: Vec<ItemImpl>,
}

impl Parse for MultiImplBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if !input.peek(Token![for]) {
            return Err(input.error("Should be started from \'for\'"));
        }
        let r#for = input.parse::<Token![for]>()?;

        let mut generics = input.parse::<Generics>()?;
        let ident = input.parse::<Path>()?;

        generics.where_clause = input.parse()?;

        let block = input.parse::<Block>()?;

        let mut stmts = Vec::with_capacity(block.stmts.len());
        for stmt in block.stmts {
            let mut item = syn::parse2::<ItemImpl>({
                match stmt {
                    syn::Stmt::Item(syn::Item::Impl(item)) => item.into_token_stream(),
                    syn::Stmt::Item(syn::Item::Verbatim(ts)) => ts,
                    item => return Err(Error::new(item.span(), "Supports only impl blocks")),
                }
            })?;
            item.generics.params.extend(generics.params.clone());
            if let Some(_where_clause) = generics.where_clause.as_ref() {
                if let Some(where_clause) = item.generics.where_clause.as_mut() {
                    where_clause.predicates.extend(_where_clause.predicates.clone())
                } else {
                    item.generics.where_clause = Some(_where_clause.clone())
                }
            }
            let self_ty = item.self_ty.as_mut();
            if !matches!(self_ty, syn::Type::Path(syn::TypePath{ path, ..}) if path.is_ident("Self")) {
                item.trait_ = Some((None, parse_quote!(#self_ty), r#for));
            }

            item.self_ty = parse_quote!(#ident);

            stmts.push(item);
        }

        Ok(Self { stmts })
    }
}

impl ToTokens for MultiImplBlock {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for item in &self.stmts {
            tokens.append_all(item.attrs.iter().filter(|a| matches!(a.style, AttrStyle::Outer)));
            item.defaultness.to_tokens(tokens);
            item.unsafety.to_tokens(tokens);
            item.impl_token.to_tokens(tokens);
            item.generics.to_tokens(tokens);
            if let Some((const_0, const_1)) = item.constantly.as_ref() {
                tokens.extend(quote::quote!(#const_0));
                tokens.extend(quote::quote!(#const_1));
            }
            if let Some((polarity, path, for_token)) = &item.trait_ {
                polarity.to_tokens(tokens);
                path.to_tokens(tokens);
                for_token.to_tokens(tokens);
            }
            item.self_ty.to_tokens(tokens);
            item.generics.where_clause.to_tokens(tokens);
            item._brace_token.surround(tokens, |tokens| {
                tokens.append_all(item.attrs.iter().filter(|a| matches!(a.style, AttrStyle::Inner(_))));
                tokens.append_all(&item.items);
            });
        }
    }
}
