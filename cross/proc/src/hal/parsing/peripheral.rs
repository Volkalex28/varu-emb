use super::*;

#[derive(Debug, Parse)]
pub struct PeripheralItem {
    #[parse(syn::Attribute::parse_outer)]
    pub(super) attrs: Vec<syn::Attribute>,

    pub(super) ident: syn::Ident,

    #[parse(Self::parse_bounds)]
    pub(super) bounds: Punctuated<syn::TypeParamBound, Token![+]>,

    pub(super) _semi: Token![;],
}
impl PeripheralItem {
    fn parse_bounds(input: parse::ParseStream) -> Result<Punctuated<syn::TypeParamBound, Token![+]>> {
        if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            input.call(Punctuated::parse_separated_nonempty)
        } else {
            Ok(Punctuated::new())
        }
    }
}

#[derive(Debug, Parse)]
pub struct Peripheral {
    pub(super) _token: tokens::peripheral,
    pub(super) _colon: Token![:],

    #[syn(braced)]
    pub(super) _brace: token::Brace,

    #[syn(in = _brace)]
    #[parse(parse_vectored)]
    pub(super) items: Vec<PeripheralItem>,
}

pub(super) mod cross {
    use super::*;

    #[derive(Debug, Parse)]
    pub struct Config {
        _token: tokens::config,
        _colon: Token![:],
        pub config: syn::Type,
        _coma: Option<Token![,]>,
    }

    #[derive(Debug, Parse)]
    pub enum ErrorType {
        #[parse(peek = Token![=])]
        Eq { _eq: Token![=], value: syn::Type },
        #[parse(peek = Token![:])]
        Traits {
            _colon: Token![:],
            #[parse(Punctuated::parse_separated_nonempty)]
            value: Punctuated<syn::TypeParamBound, Token![+]>,
        },
    }

    #[derive(Debug, Parse)]
    pub struct Error {
        _token: tokens::error,
        pub ty: ErrorType,
        _coma: Option<Token![,]>,
    }

    #[derive(Debug, Parse)]
    pub struct Cross {
        pub config: Config,

        #[parse(|input| input.peek(tokens::error).then(|| input.parse()).transpose() )]
        pub error: Option<Error>,
    }

    mod tokens {
        syn::custom_keyword!(config);
        syn::custom_keyword!(error);
    }
}
