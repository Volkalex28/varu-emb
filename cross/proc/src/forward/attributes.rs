use super::tokens;
use crate::Parenthesized;
use syn::parse::{Nothing, Parse};
use syn::punctuated::Punctuated;
use syn::token;
use syn_derive::Parse;

pub type Members = Punctuated<Member, token::Comma>;

#[derive(Debug, Parse)]
pub enum Property<D: Parse = Nothing> {
    #[parse(peek = tokens::error)]
    Error {
        token: tokens::error,
        #[allow(unused)]
        colon: token::Colon,
        ty: syn::Type,
    },

    #[parse(peek = token::Async)]
    Async {
        token: token::Async,
        #[parse(Parenthesized::parse_opt)]
        data: Option<Parenthesized<D>>,
    },

    #[parse(peek = tokens::blocking)]
    Blocking {
        token: tokens::blocking,
        #[parse(Parenthesized::parse_opt)]
        data: Option<Parenthesized<D>>,
    },
}

#[derive(Debug, Parse)]
pub enum Interface {
    #[parse(peek = tokens::gpio)]
    Gpio {
        #[allow(unused)]
        token: tokens::gpio,
        props: Parenthesized<Property<gpio::Part>>,
    },

    #[parse(peek = tokens::i2c)]
    I2c {
        #[allow(unused)]
        token: tokens::i2c,
        props: Parenthesized<Property>,
    },

    #[parse(peek = tokens::io)]
    Io {
        #[allow(unused)]
        token: tokens::io,
        props: Parenthesized<Property<io::Part>>,
    },

    #[parse(peek = tokens::spi)]
    Spi {
        #[allow(unused)]
        token: tokens::spi,
        props: Parenthesized<Property<spi::Type>>,
    },
}

#[derive(Debug, Parse)]
pub struct Member {
    pub ident: syn::Member,
    #[allow(unused)]
    pub colon: token::Colon,
    pub interface: Interface,
}

pub mod gpio {
    use super::*;

    #[derive(Debug, Parse)]
    pub enum Part {
        #[parse(peek = tokens::input)]
        Input(tokens::input),
        #[parse(peek = tokens::output)]
        Output(tokens::output),
        #[parse(peek = tokens::stateful)]
        Stateful(tokens::stateful),
    }

    mod tokens {
        syn::custom_keyword!(input);
        syn::custom_keyword!(output);
        syn::custom_keyword!(stateful);
    }
}

pub mod io {
    use super::*;

    #[derive(Debug, Parse)]
    pub enum Part {
        #[parse(peek = tokens::read)]
        Read(tokens::read),
        #[parse(peek = tokens::write)]
        Write(tokens::write),
    }

    mod tokens {
        syn::custom_keyword!(read);
        syn::custom_keyword!(write);
    }
}

pub mod spi {
    use super::*;

    #[derive(Debug, Parse)]
    pub enum Type {
        #[parse(peek = tokens::bus)]
        Bus(tokens::bus),
        #[parse(peek = tokens::device)]
        Device(tokens::device),
    }

    mod tokens {
        syn::custom_keyword!(bus);
        syn::custom_keyword!(device);
    }
}
