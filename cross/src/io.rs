pub use embedded_io::*;

pub use supply::{IoErrorType as ErrorType, IoRead as Read, IoWrite as Write};

pub use supply::{
    uncurry_trait_forwarding_info_for_IoErrorType as uncurry_trait_forwarding_info_for_ErrorType,
    uncurry_trait_forwarding_info_for_IoRead as uncurry_trait_forwarding_info_for_Read,
    uncurry_trait_forwarding_info_for_IoWrite as uncurry_trait_forwarding_info_for_Write,
};

pub mod asynch {
    pub use super::*;

    pub use supply::{IoAsyncRead as Read, IoAsyncWrite as Write};

    pub use supply::{
        uncurry_trait_forwarding_info_for_IoAsyncRead as uncurry_trait_forwarding_info_for_Read,
        uncurry_trait_forwarding_info_for_IoAsyncWrite as uncurry_trait_forwarding_info_for_Write,
    };
}

mod alias {
    pub use embedded_io::{ErrorType as IoErrorType, Read as IoRead, Write as IoWrite};
    pub use embedded_io_async::{Read as IoAsyncRead, Write as IoAsyncWrite};
}

mod supply {
    use super::alias;
    use forward_traits::supply_forwarding_info_for_trait;

    supply_forwarding_info_for_trait! {
        alias::IoErrorType,
        pub trait {
            type Error;
        }
    }

    supply_forwarding_info_for_trait! {
        alias::IoRead,
        pub trait {
            #[inline]
            fn read(&mut self, buf: &mut [::core::primitive::u8])
                -> ::core::result::Result<::core::primitive::usize, Self::Error>;
            #[inline]
            fn read_exact(&mut self, buf: &mut [::core::primitive::u8])
                -> ::core::result::Result<(), ::varuemb::cross::io::ReadExactError<Self::Error>>;
        }
    }

    supply_forwarding_info_for_trait! {
        alias::IoWrite,
        pub trait {
            #[inline]
            fn write(&mut self, buf: &[::core::primitive::u8])
                -> ::core::result::Result<::core::primitive::usize, Self::Error>;
            #[inline]
            fn flush(&mut self)
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            fn write_all(&mut self, buf: &[::core::primitive::u8])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            fn write_fmt(&mut self, fmt: ::core::fmt::Arguments<'_>)
                -> ::core::result::Result<(), ::varuemb::cross::io::WriteFmtError<Self::Error>>;
        }
    }

    supply_forwarding_info_for_trait! {
        alias::IoAsyncRead,
        pub trait {
            #[inline]
            async fn read(&mut self, buf: &mut [::core::primitive::u8])
                -> ::core::result::Result<usize, Self::Error>;
            #[inline]
            async fn read_exact(&mut self, buf: &mut [::core::primitive::u8])
                -> ::core::result::Result<(), ::varuemb::cross::io::ReadExactError<Self::Error>>;
        }
    }

    supply_forwarding_info_for_trait! {
        alias::IoAsyncWrite,
        pub trait {
            #[inline]
            async fn write(&mut self, buf: &[::core::primitive::u8])
                -> ::core::result::Result<usize, Self::Error>;
            #[inline]
            async fn flush(&mut self)
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            async fn write_all(&mut self, buf: &[::core::primitive::u8])
                -> ::core::result::Result<(), Self::Error>;
        }
    }
}

pub mod __private {
    use super::{ReadExactError, WriteFmtError};

    pub fn read_exact_error<E0, E1: From<E0>>(err: ReadExactError<E0>) -> ReadExactError<E1> {
        match err {
            ReadExactError::UnexpectedEof => ReadExactError::UnexpectedEof,
            ReadExactError::Other(e) => ReadExactError::Other(e.into()),
        }
    }

    pub fn write_fmt_error<E0, E1: From<E0>>(err: WriteFmtError<E0>) -> WriteFmtError<E1> {
        match err {
            WriteFmtError::FmtError => WriteFmtError::FmtError,
            WriteFmtError::Other(e) => WriteFmtError::Other(e.into()),
        }
    }
}
