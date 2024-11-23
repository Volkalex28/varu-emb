#![feature(cfg_version)]
#![cfg_attr(not(version("1.79")), feature(associated_type_bounds))]

use heck::ToSnakeCase;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::parse::*;
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput, Token};

#[derive(Debug)]
enum Process {
    Path(syn::Path),
    Closure(syn::ExprClosure),
}
impl Parse for Process {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![|]) {
            input.parse::<syn::ExprClosure>().map(Self::Closure)
        } else {
            input.parse::<syn::Path>().map(Self::Path)
        }
    }
}
impl quote::ToTokens for Process {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Process::Path(path) => path.to_tokens(tokens),
            Process::Closure(closure) => closure.to_tokens(tokens),
        }
    }
}

#[derive(Debug)]
struct Task {
    alias: Option<Ident>,
    generics: Option<syn::Generics>,
    process: Option<Process>,
    infinity: syn::LitBool,
    count: syn::Expr,
    link_section: Option<syn::LitStr>,
}
impl Default for Task {
    fn default() -> Self {
        Self {
            alias: None,
            generics: None,
            process: None,
            infinity: syn::parse_quote!(true),
            count: syn::parse_quote!(1usize),
            link_section: None,
        }
    }
}
impl Task {
    fn new(attrs: Vec<syn::Attribute>) -> syn::Result<Self> {
        let Some(attr) = attrs.into_iter().find(|attr| attr.path.is_ident("task")) else {
            return Ok(Self::default());
        };
        syn::parse2(attr.tokens)
    }
}
impl Parse for Task {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        use task_tokens as tokens;

        let mut alias = None;
        let mut generics = None;
        let mut process = None;
        let mut infinity = None;
        let mut count = None;
        let mut link_section = None;

        let content;
        let _t = syn::parenthesized!(content in input);

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if parse_one(&mut alias, &lookahead, &content, tokens::alias)? {
            } else if parse_one(&mut generics, &lookahead, &content, tokens::generics)? {
            } else if parse_one(&mut count, &lookahead, &content, tokens::count)? {
            } else if parse_one(&mut infinity, &lookahead, &content, tokens::infinity)? {
            } else if parse_one(&mut link_section, &lookahead, &content, tokens::link_section)? {
            } else if parse_one(&mut process, &lookahead, &content, tokens::process)? {
            } else {
                return Err(lookahead.error());
            }
        }

        let default = Self::default();
        Ok(Self {
            alias,
            generics,
            process,
            infinity: infinity.unwrap_or(default.infinity),
            count: count.unwrap_or(default.count),
            link_section,
        })
    }
}

mod task_tokens {
    syn::custom_keyword!(alias);
    syn::custom_keyword!(generics);
    syn::custom_keyword!(count);
    syn::custom_keyword!(infinity);
    syn::custom_keyword!(process);
    syn::custom_keyword!(link_section);
}

#[proc_macro_derive(Task, attributes(task))]
pub fn implementation(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let task = match Task::new(input.attrs) {
        Ok(task) => task,
        Err(err) => return err.into_compile_error().into(),
    };

    let ident = input.ident;
    let count = task.count;
    let alias = task.alias.unwrap_or(ident.clone());
    let new_ident = Ident::new(&(alias.to_string() + "Task"), ident.span());
    let r#impl = {
        let mut ret = TokenStream::default();
        if !input.generics.params.is_empty() {
            let Some(gen) = task.generics else {
                let err = syn::Error::new(input.generics.span(), "Attribute \"task::generics\" not found");
                return err.into_compile_error().into();
            };

            for generic in input.generics.params.iter() {
                use syn::GenericParam::*;
                let (attr, bounds) = match generic {
                    Type(ty) => (
                        ty.ident.to_string(),
                        gen.type_params().find_map(|gen| (gen.ident == ty.ident).then_some(&gen.bounds)).map(|b| quote!(#b)),
                    ),
                    Lifetime(life) => (
                        life.lifetime.to_string(),
                        gen.lifetimes()
                            .find_map(|gen| (gen.lifetime == life.lifetime).then_some(&gen.bounds))
                            .map(|b| quote!(#b)),
                    ),
                    Const(cons) => (
                        cons.ident.to_string(),
                        gen.const_params()
                            .find_map(|gen| (gen.ident == cons.ident).then_some(&gen.default))
                            .map(|b| quote!(#b)),
                    ),
                };
                if bounds.is_none() {
                    let err = syn::Error::new(generic.span(), format!("Generic param {attr} not found"));
                    return err.into_compile_error().into();
                }
                ret.extend(quote!(#bounds,))
            }
        }
        let ret = (!ret.is_empty()).then_some(quote!(< #ret >));
        quote!(pub type #new_ident = #ident #ret ; )
    };
    let (gen, ty, wh) = input.generics.split_for_impl();

    let func = task.process.unwrap_or(syn::parse_quote!(#new_ident :: process));

    let name = alias.to_string();
    let link_section = task.link_section.map(|section| {
        let section = section.value().to_string() + "." + &name + "_task";
        quote!(#[link_section = #section])
    });

    let span = proc_macro2::Span::mixed_site();
    let ident_lower = alias.to_string().to_snake_case();
    let log = Ident::new(&(ident_lower.clone() + "_log"), span);
    let error = Ident::new(&(ident_lower.clone() + "_error"), span);
    let warn = Ident::new(&(ident_lower.clone() + "_warn"), span);
    let info = Ident::new(&(ident_lower.clone() + "_info"), span);
    let debug = Ident::new(&(ident_lower + "_debug"), span);

    let body = (!task.infinity.value())
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

    let out = quote!(
        #[allow(unused)]
        use ::varuemb::executor::Task as _;
        #[allow(unused)]
        use ::varuemb::executor::TaskName as _;
        #[allow(unused)]
        #r#impl

        const _: () = {
        pub trait _Internal {
            type Task;
            type Fut: ::core::future::Future<Output = Self::Output> + 'static;
            type Output: ::core::fmt::Debug + 'static;
            fn entry(this: Self::Task) -> Self::Fut;
        }
        impl _Internal for () {
            type Task = #new_ident;
            type Fut = impl ::core::future::Future<Output: ::core::fmt::Debug> + 'static;
            type Output = <Self::Fut as ::core::future::Future>::Output;
            #[inline]
            fn entry(this: Self::Task) -> Self::Fut {
                (#func)(this)
            }
        }
        impl ::varuemb::executor::Task for #new_ident {
            type Fut = <() as _Internal>::Fut;

            fn process(self) -> Self::Fut { <() as _Internal>::entry(self) }

            fn finish(res: <Self::Fut as ::core::future::Future>::Output) { #body }

            fn pool() -> ::varuemb::executor::task::PoolRef<Self> {
                #link_section
                static POOL: ::varuemb::executor::task::Pool<#new_ident, {#count}> = ::varuemb::executor::task::Pool::new();
                POOL.as_ref()
            }
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
    );
    out.into()
}

fn parse_one<T: Peek<Token: Parse + Spanned>, R: Parse>(
    ret: &mut Option<R>,
    lookaheed: &Lookahead1,
    input: &ParseBuffer,
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
