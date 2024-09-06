pub use super::*;
pub use supply::SpiDevice;

pub use supply::uncurry_trait_forwarding_info_for_SpiDevice;

pub mod asynch {
    pub use super::*;

    pub use supply::uncurry_trait_forwarding_info_for_AsyncSpiDevice as uncurry_trait_forwarding_info_for_SpiDevice;
    pub use supply::AsyncSpiDevice as SpiDevice;
}

mod alias {
    pub use embedded_hal::spi::SpiDevice;
    pub use embedded_hal_async::spi::SpiDevice as AsyncSpiDevice;
}

mod supply {
    use super::alias;
    use forward_traits::supply_forwarding_info_for_trait;

    supply_forwarding_info_for_trait! {
        alias::SpiDevice,
        pub trait <Word: ::core::marker::Copy + 'static = ::core::primitive::u8> {
            #[inline]
            fn transaction(&mut self, operations: &mut [::varuemb::cross::spi::Operation<'_, Word>])
                -> ::core::result::Result<(), Self::Error>;
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
        }
    }

    supply_forwarding_info_for_trait! {
        alias::AsyncSpiDevice,
        pub trait <Word: ::core::marker::Copy + 'static = ::core::primitive::u8> {
            #[inline]
            async fn transaction(&mut self, operations: &mut [::varuemb::cross::spi::Operation<'_, Word>])
                -> ::core::result::Result<(), Self::Error>;
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
        }
    }
}
