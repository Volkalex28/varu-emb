use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, Error, ItemStruct};
use varuemb_utils::proc_meta_parser::Parser;

pub struct Notifier<'a> {
    meta: &'a super::Meta,
    attrs: TokenStream,
    item: &'a ItemStruct,
}

impl<'a> Notifier<'a> {
    pub(crate) fn new(
        attrs: TokenStream,
        input: &'a ItemStruct,
        meta: &'a super::Meta,
    ) -> Result<Self, Error> {
        for field in input.fields.iter() {
            for attr in field.attrs.iter() {
                if !attr.path().is_ident("notifier_service")
                    && !attr.path().is_ident("cfg")
                    && !attr.path().is_ident("cfg_attr")
                {
                    return Err(Error::new(
                        attr.span(),
                        "Supports only \"notifier_service\" or \"cfg\" attribute per field",
                    ));
                }
            }
            if field.ident.is_none() {
                return Err(Error::new(
                    field.span(),
                    "Supports only named fields (e.g: name: Service)",
                ));
            }
        }

        Ok(Self {
            meta,
            attrs,
            item: input,
        })
    }
}

impl<'a> ToTokens for Notifier<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let _crate = &self.meta.crate_ident;
        let ident = &self.item.ident;
        let self_fields = &self.item.fields;

        let fields_attrs = self_fields
            .iter()
            .map(|field| {
                field
                    .attrs
                    .iter()
                    .filter(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"))
                    .flat_map(|attr| quote!(#attr))
                    .collect::<TokenStream>()
            })
            .collect::<Vec<_>>();

        // Notifier
        tokens.extend({
            let vis = &self.item.vis;
            let attrs = &self.item.attrs;
            let fields = self_fields
                .iter()
                .enumerate()
                .flat_map(|(i, field)| {
                    let ident = &field.ident;
                    let ty = &field.ty;
                    let attrs = &fields_attrs[i];
                    quote!(
                        #attrs
                        #ident: #_crate ::GetService<Self, #ty>,
                    )
                })
                .collect::<TokenStream>();

            quote! {
                #(#attrs)*
                #vis struct #ident {
                    #fields
                }
            }
        });

        //Impl Notifier
        tokens.extend({
            let attrs = &self.attrs;
            let (count, fields): (TokenStream, TokenStream) = self_fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    let ty = &field.ty;
                    let ident = &field.ident;
                    let f_attrs = &fields_attrs[i];
                    let count = quote!(
                        #f_attrs
                        let __count = __count + #_crate ::count::<Self, #ty>();
                    );
                    let attrs = &fields_attrs[i];
                    let field = quote!(
                        #attrs
                        #ident: ::core::default::Default::default(),
                    );
                    (count, field)
                })
                .unzip();
            let count_services = self
                .item
                .fields
                .iter()
                .enumerate()
                .flat_map(|(i, _)| {
                    let attrs = &fields_attrs[i];
                    quote! {
                        #attrs
                        let __count = __count + 1;
                    }
                })
                .collect::<TokenStream>();
            quote! {
                impl #_crate ::traits::Notifier for #ident {
                    const COUNT: ::core::primitive::usize = {
                        let __count = 0;
                        #count
                        __count
                    };
                    const COUNT_SERVICES: ::core::primitive::usize = {
                        let __count = 0;
                        #count_services
                        __count
                    };

                    fn get() -> &'static Self {
                        #attrs
                        static __THIS__: #ident = #ident {
                            #fields
                        };
                        &__THIS__
                    }
                }
            }
        });

        //Impl NotifierService
        tokens.extend(self_fields.iter().enumerate().flat_map(|(id, field)| {
            let field_ident = &field.ident.as_ref().unwrap();
            let attrs = &fields_attrs[id];
            let ty = &field.ty;
            let name = field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("notifier_service"))
                .map_or(
                    field_ident
                        .to_string()
                        .to_upper_camel_case()
                        .to_token_stream(),
                    |a| {
                        let mut parser = Parser::new(["name"], a.span());
                        if let Err(err) = a.parse_nested_meta(|meta| parser.parse(meta)) {
                            return err.to_compile_error();
                        };
                        parser.get("name").unwrap()
                    },
                );
            let id = fields_attrs
                .iter()
                .take(id)
                .flat_map(|attr| {
                    quote! {
                        #attr
                        let __count = __count + 1;
                    }
                })
                .collect::<TokenStream>();
            quote! {
                #attrs
                impl #_crate ::traits::NotifierService<#ty> for #ident {
                    const ID: ::core::primitive::usize = {
                        let __count = 0;
                        #id
                        __count
                    };
                    const NAME: &'static str = #name;
                    fn __get(&self) -> #_crate::traits::NotifierServiceGetRet<Self, #ty> {
                        &self. #field_ident
                    }
                }
            }
        }));

        //Impl NotifierEvent
        tokens.extend({
            let calc = self_fields
                .iter()
                .enumerate()
                .flat_map(|(i, field)| {
                    let ty = &field.ty;
                    let attrs = &fields_attrs[i];
                    quote!(
                        #attrs
                        let __count = __count.calc::<#ty, #ident, __E>();
                    )
                })
                .collect::<TokenStream>();
            quote! {
                impl<__E> #_crate ::traits::NotifierServiceEvent<__E> for #ident
                where
                    __E: #_crate ::event::traits::Event<Self>,
                    __E::Service: #_crate ::service::traits::Service<Self>,
                    Self: #_crate ::traits::NotifierService<__E::Service>,
                    #_crate ::GetPubSub<Self, __E::Service>: #_crate ::pubsub::traits::Publisher<__E>,
                {
                    const COUNT_CALC: #_crate::calc::CountID = {
                        let __count = #_crate ::calc::CountID::default();
                        #calc
                        __count
                    };
                }
            }
        });

        //Impl NotifierPublisher
        tokens.extend({
            let calc = self_fields
                .iter()
                .enumerate()
                .flat_map(|(i, field)| {
                    let ty = &field.ty;
                    let attrs = &fields_attrs[i];
                    quote!(
                        #attrs
                        let __calc = __calc.add::<#ty>();
                    )
                })
                .collect::<TokenStream>();
            quote! {
                impl<__E> #_crate ::traits::NotifierPublisher<__E> for #ident
                where
                    __E: #_crate ::event::traits::Event<Self> + 'static,
                    __E::Service: #_crate ::service::traits::Service<Self>,
                    Self: #_crate ::traits::NotifierService<__E::Service>,
                    #_crate ::GetPubSub<Self, __E::Service>: #_crate ::pubsub::traits::Publisher<__E>,
                    [(); <Self as #_crate ::traits::NotifierServiceEvent<__E>>::ID_COUNT]:,
                {
                    const ID_CALC: #_crate ::calc::CalcID<Self, __E> = {
                        let __calc = #_crate ::calc::CalcID::<Self, __E>::default();
                        #calc
                        __calc.verify()
                    };
                }
            }
        });
    }
}
