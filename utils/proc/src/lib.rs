use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, TokenStreamExt};
use syn::{
    parse::Parse, parse_quote, spanned::Spanned, AttrStyle, Block, Error, Generics, Path, Token,
};

struct ItemImpl {
    pub attrs: Vec<syn::Attribute>,
    pub defaultness: Option<Token![default]>,
    pub unsafety: Option<Token![unsafe]>,
    pub impl_token: Token![impl],
    pub constantly: Option<(Option<Token![?]>, Token![const])>,
    pub generics: Generics,
    /// Trait this impl implements.
    pub trait_: Option<(Option<Token![!]>, Path, Token![for])>,
    /// The Self type of the impl.
    pub self_ty: Box<syn::Type>,
    pub brace_token: syn::token::Brace,
    pub items: Vec<syn::ImplItem>,
}

impl Parse for ItemImpl {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        use syn::*;

        let mut tokens = TokenStream2::new();
        let constantly = {
            tokens.extend({
                let a = input.call(Attribute::parse_outer)?;
                quote::quote!(#(#a)*)
            });
            tokens.extend(input.parse::<Visibility>()?.to_token_stream());
            tokens.extend(input.parse::<Option<Token![default]>>()?.to_token_stream());
            tokens.extend(input.parse::<Option<Token![unsafe]>>()?.to_token_stream());
            tokens.extend(input.parse::<Token![impl]>()?.to_token_stream());
            
            if input.peek(Token![<]) {
                tokens.extend(input.parse::<Generics>()?.to_token_stream());
            }

            let is_const_impl =
                input.peek(Token![const]) || input.peek(Token![?]) && input.peek2(Token![const]);
            let constantly = if is_const_impl {
                let const_0 = input.parse::<Option<Token![?]>>()?;
                let const_1 = input.parse::<Token![const]>()?;
                Some((const_0, const_1))
            } else {
                None
            };

            tokens.extend(input.cursor().token_stream());
            _ = input.step(|s| {
                let mut rest = *s;
                while let Some((_, next)) = rest.token_tree() {
                    rest = next
                }
                Ok(((), rest))
            });

            constantly
        };

        let item = parse2::<ItemImpl>(tokens)?;

        Ok(Self {
            attrs: item.attrs,
            defaultness: item.defaultness,
            unsafety: item.unsafety,
            impl_token: item.impl_token,
            constantly,
            generics: item.generics,
            trait_: item.trait_,
            self_ty: item.self_ty,
            brace_token: item.brace_token,
            items: item.items,
        })
    }
}

struct MultiImplBlock {
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
                let t = match stmt {
                    syn::Stmt::Item(syn::Item::Impl(item)) => item.into_token_stream(),
                    syn::Stmt::Item(syn::Item::Verbatim(ts)) => ts,
                    item => return Err(Error::new(item.span(), "Supports only impl blocks")),
                };
                t
            })?;
            item.generics.params.extend(generics.params.clone());
            if let Some(_where_clause) = generics.where_clause.as_ref() {
                if let Some(where_clause) = item.generics.where_clause.as_mut() {
                    where_clause
                        .predicates
                        .extend(_where_clause.predicates.clone())
                } else {
                    item.generics.where_clause = Some(_where_clause.clone())
                }
            }
            let self_ty = item.self_ty.as_mut();
            if !matches!(self_ty, syn::Type::Path(syn::TypePath{ path, ..}) if path.is_ident("Self"))
            {
                item.trait_ = Some((None, parse_quote!(#self_ty), r#for.clone()));
            }

            item.self_ty = parse_quote!(#ident);

            stmts.push(item);
        }

        Ok(Self { stmts })
    }
}

impl ToTokens for MultiImplBlock {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        for item in &self.stmts {
            tokens.append_all(
                item.attrs
                    .iter()
                    .filter(|a| matches!(a.style, AttrStyle::Outer)),
            );
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
            item.brace_token.surround(tokens, |tokens| {
                tokens.append_all(
                    item.attrs
                        .iter()
                        .filter(|a| matches!(a.style, AttrStyle::Inner(_))),
                );
                tokens.append_all(&item.items);
            });
        }
    }
}

#[proc_macro]
pub fn multi_impl_block(input: TokenStream) -> TokenStream {
    (|| -> Result<TokenStream2, Error> {
        Ok(syn::parse::<MultiImplBlock>(input)?.into_token_stream())
    })()
    .map_err(|err| err.to_compile_error())
    .map_or_else(Into::into, Into::into)
}
