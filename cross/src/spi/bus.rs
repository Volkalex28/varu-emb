pub use super::*;
pub use supply::SpiBus;

pub use supply::uncurry_trait_forwarding_info_for_SpiBus;

pub mod asynch {
    pub use super::*;

    pub use supply::uncurry_trait_forwarding_info_for_AsyncSpiBus as uncurry_trait_forwarding_info_for_SpiBus;
    pub use supply::AsyncSpiBus as SpiBus;
}

mod alias {
    pub use embedded_hal::spi::SpiBus;
    pub use embedded_hal_async::spi::SpiBus as AsyncSpiBus;
}

mod supply {
    use super::alias;
    use forward_traits::supply_forwarding_info_for_trait;

    supply_forwarding_info_for_trait! {
        alias::SpiBus,
        pub trait <Word: ::core::marker::Copy + 'static = ::core::primitive::u8> {
            #[inline]
            fn read(&mut self, buf: &mut [Word])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            fn write(&mut self, buf: &[Word])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            fn transfer(&mut self, read: &mut [Word], write: &[Word])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            fn transfer_in_place(&mut self, buf: &mut [Word])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            fn flush(&mut self)
                -> ::core::result::Result<(), Self::Error>;
        }
    }

    supply_forwarding_info_for_trait! {
        alias::AsyncSpiBus,
        pub trait <Word: ::core::marker::Copy + 'static = ::core::primitive::u8> {
            #[inline]
            async fn read(&mut self, buf: &mut [Word])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            async fn write(&mut self, buf: &[Word])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            async fn transfer(&mut self, read: &mut [Word], write: &[Word])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            async fn transfer_in_place(&mut self, buf: &mut [Word])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            async fn flush(&mut self)
                -> ::core::result::Result<(), Self::Error>;
        }
    }
}
