use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse, Error, Result};
use syn_derive::Parse;

#[derive(Debug, Parse)]
enum Attribute {
    #[parse(peek = tokens::task)]
    Task(crate::ParenAttribute<tokens::task, syn::Type>),
    #[parse(peek = tokens::statistic)]
    Statistic(#[allow(unused)] tokens::statistic),
}

pub struct Execution {
    ident: syn::Ident,
    generics: syn::Generics,
    statistic: Option<syn::Member>,
    tasks: Vec<(syn::Member, syn::Type)>,
}
impl Execution {
    fn new(ident: syn::Ident, generics: syn::Generics) -> Self {
        Self { ident, generics, statistic: None, tasks: Vec::new() }
    }
}
impl parse::Parse for Execution {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let input = input.parse::<syn::ItemStruct>()?;
        let mut this = Self::new(input.ident, input.generics);

        for (i, mut field) in input.fields.into_iter().enumerate() {
            let Some(attr) = field.attrs.pop_if(|attr| attr.path().is_ident("varuemb_executor")) else {
                continue;
            };

            let attribute = attr.parse_args::<Attribute>()?;
            match attribute {
                Attribute::Task(task) => this.tasks.push((map_ident(i, field.ident), task.content)),
                Attribute::Statistic(_) if this.statistic.is_none() => this.statistic = map_ident(i, field.ident).into(),
                Attribute::Statistic(_) => {
                    return Err(Error::new(field.span(), "Duplicate statistic field"));
                }
            }
        }

        Ok(this)
    }
}
impl ToTokens for Execution {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = &self.ident;
        let (g_impl, g_types, g_where) = self.generics.split_for_impl();

        if let Some(statistic) = self.statistic.as_ref() {
            tokens.extend(quote! {
                impl #g_impl ::core::convert::AsRef<::varuemb::executor::statistic::Statistic> for #ident #g_types #g_where {
                    fn as_ref(&self) -> &::varuemb::executor::statistic::Statistic {
                        &self. #statistic
                    }
                }
            });
        }

        for (field, ty) in &self.tasks {
            tokens.extend(quote! {
                impl #g_impl ::varuemb::executor::PoolProvider<#ty> for #ident #g_types #g_where {
                    fn pool(&self) -> varuemb::executor::task::PoolRef< #ty > {
                        self. #field .as_ref()
                    }
                }
            });
        }
    }
}

fn map_ident(i: usize, ident: Option<syn::Ident>) -> syn::Member {
    match ident {
        Some(ident) => syn::Member::Named(ident),
        None => syn::Member::Unnamed(syn::parse_quote!(#i)),
    }
}

mod tokens {
    syn::custom_keyword!(statistic);
    syn::custom_keyword!(task);
}
