use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, DeriveInput, Error, Ident};

pub mod data;

#[derive(Debug)]
pub struct Service<'a> {
    ident: &'a Ident,
    meta: &'a super::Meta,
    data: data::Data,
}

impl<'a> Service<'a> {
    pub(crate) fn new(input: &'a DeriveInput, meta: &'a super::Meta) -> Result<Self, Error> {
        let ident = &input.ident;
        let mut data = data::Data::new(&input.attrs)?;

        if let Some(rpc) = data.rpc.as_ref() {
            let _crate = &meta.crate_ident;
            data.subscribers.insert(
                syn::parse_quote!(#_crate ::rpc::GetRequest<Self, #ident>),
                data::SubscriberData {
                    count: rpc.clone(),
                    name: Ident::new("_rpc", rpc.span()),
                    mixed: None,
                    ty: data::SubscriberType::PubSub,
                },
            );
        }

        Ok(Self { meta, data, ident })
    }

    fn generate(&self) -> TokenStream {
        let _impl = Ident::new("__impl", Span::mixed_site());
        let _crate = &self.meta.crate_ident;
        let _notif = &self.data.notifier;
        let _ident = self.ident;

        let mut out = TokenStream::default();

        // Service
        out.extend({
            let _count = self
                .data
                .count
                .as_ref()
                .map_or(quote!(1usize), |count| quote!(#count));
            quote! {
                impl #_crate ::service::traits::Service<#_notif> for #_ident {
                    const COUNT: ::core::primitive::usize = #_count;
                    type Impl = #_impl;
                }
            }
        });

        // Impl
        out.extend({
            let fields = self
                .data
                .subscribers
                .iter()
                .flat_map(|(path, data)| {
                    let count = &data.count;
                    let name = &data.name;
                    let ty = match &data.ty {
                        data::SubscriberType::PubSub => {
                            quote!(#_crate ::pubsub::Subscription<Self, #path, { #count }>)
                        }
                        data::SubscriberType::Rpc => {
                            quote!(#_crate ::rpc::Subscription<Self, #path, { #count }>)
                        }
                    };
                    quote! { #name: #ty,}
                })
                .collect::<TokenStream>();
            quote! {
                #[allow(non_camel_case_types)]
                pub struct #_impl {
                    #fields
                }
            }
        });

        // Impl PubSub
        out.extend({
            let fields = self
                .data
                .subscribers
                .iter()
                .flat_map(|(_, data)| {
                    let name = &data.name;
                    quote! { #name: ::core::default::Default::default(),}
                })
                .collect::<TokenStream>();
            quote! {
                impl const #_crate ::pubsub::traits::PubSub for #_impl {
                    type Service = #_ident;
                    type Notifier = #_notif;
                    fn __new() -> Self {
                        Self { #fields }
                    }
                }
            }
        });

        // Impl Publisher
        out.extend(
            self.data
                .publishers
                .iter()
                .flat_map(|(path, data)| {
                    let protected = data.protected.as_ref().map(|value| {
                        quote!(
                            const PROTECTED: ::core::primitive::bool = #value;
                        )
                    });
                    quote! {
                        impl #_crate ::pubsub::traits::Publisher<#path> for #_impl {
                            #protected
                        }
                    }
                })
                .collect::<TokenStream>(),
        );

        // Impl Subscriber
        out.extend({
            self.data
                .subscribers
                .iter()
                .flat_map(|(path, data)| {
                    let ret = match &data.ty {
                        data::SubscriberType::PubSub => quote!(#_crate ::pubsub::traits::GetSubscriberRet<Self::Notifier, #path>),
                        data::SubscriberType::Rpc => quote!(#_crate ::rpc::traits::GetSubscriberRet<Self::Notifier, #path>),
                    };
                    let path = match &data.ty {
                        data::SubscriberType::PubSub => path.to_token_stream(),
                        data::SubscriberType::Rpc => {
                            quote!(#_crate ::rpc::GetResponse<Self, #path>)
                        }
                    };
                    let name = &data.name;
                    let field = match &data.ty {
                        data::SubscriberType::PubSub => quote!(&self. #name),
                        data::SubscriberType::Rpc => quote!(&*self. #name),
                    };
                    quote! {
                        impl #_crate ::pubsub::traits::Subscriber<#path> for #_impl {
                            const IMPL: ::core::primitive::bool = true;
                            fn __get(&'static self) -> #ret {
                                ::core::convert::Into::into(#field)
                            }
                        }
                    }
                })
                .collect::<TokenStream>()
        });

        let mixed = self
            .data
            .subscribers
            .values()
            .any(|data| data.mixed.is_some());
        if !mixed {
            return out;
        }

        let mix_iter = self
            .data
            .subscribers
            .iter()
            .filter(|(_, data)| data.mixed.is_some());

        // __Mixed
        out.extend({
            let fields = mix_iter
                .clone()
                .flat_map(|(path, data)| {
                    let name = &data.name;
                    quote!(#name: #_crate ::pubsub::mixer::MixerMapper<#_impl, M, #path>,)
                })
                .collect::<TokenStream>();
            quote! {
                pub struct __Mixed<M: #_crate ::pubsub::mixer::Mixer<#_notif>> {
                    #fields
                }
            }
        });

        // Impl SubscriberMixer
        out.extend({
            let calc = mix_iter
                .clone()
                .flat_map(|(path, _)| {
                    quote!(.calc::<#path>())
                })
                .collect::<TokenStream>();
            let new = mix_iter
                .clone()
                .flat_map(|(path, data)| {
                    let name = &data.name;
                    quote!(#name: #_crate ::pubsub::mixer::MixerMapper::<#_impl, M, #path>::new(self),)
                })
                .collect::<TokenStream>();
            let mixed = mix_iter
                .clone()
                .flat_map(|(_, data)| {
                    let name = &data.name;
                    quote!( __e = __mixed.#name.map() => { __e } )
                })
                .collect::<TokenStream>();
            let try_mixed = mix_iter
                .clone()
                .flat_map(|(_, data)| {
                    let name = &data.name;
                    quote!( .or_else(|| __mixed.#name.try_map()) )
                })
                .collect::<TokenStream>();

            quote! {
                impl<M> #_crate ::pubsub::mixer::SubscriberMixer<M> for #_impl
                where
                    M: #_crate ::pubsub::mixer::Mixer<#_notif>,
                {
                    const COUNT_CALC: #_crate ::pubsub::mixer::MixCount<Self, M> = #_crate ::pubsub::mixer::MixCount::default() #calc;
                    type Mixed = __Mixed<M>;

                    fn __new_mixed(&'static self) -> Self::Mixed {
                        __Mixed { #new }
                    }

                    async fn __mixed(__mixed: &mut Self::Mixed) -> #_crate ::event::Event<Self::Notifier, M> {
                        #_crate ::select! { #mixed }
                    }

                    fn __try_mixed(__mixed: &mut Self::Mixed) -> Option<#_crate ::event::Event<Self::Notifier, M>> {
                        None #try_mixed
                    }
                }
            }
        });

        // Impl MixMapper
        out.extend(mix_iter.flat_map(|(path, data)| {
            let (mixer, mix_mapper) = data.mixed.as_ref().unwrap();
            quote! {
                impl #_crate ::pubsub::mixer::MixMapper<#mixer, #path> for #_impl {
                    type Data = #_crate ::pubsub::mixer::MixData<Self, #mixer, #path>;
                    const MAPPER: fn(#path) -> #mixer = #mix_mapper;
                }
            }
        }));

        out
    }
}

impl<'a> ToTokens for Service<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let _crate = &self.meta.crate_ident;
        let out = self.generate();
        tokens.extend(quote! {
            use #_crate ::service::traits::Service as _;
            const _: () = { #out }; 
        })
    }
}
