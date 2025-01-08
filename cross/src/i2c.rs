pub use embedded_hal::i2c::*;

pub use supply::{I2c, I2cErrorType as ErrorType};

pub use supply::{
    uncurry_trait_forwarding_info_for_I2c,
    uncurry_trait_forwarding_info_for_I2cErrorType as uncurry_trait_forwarding_info_for_ErrorType,
};

pub mod asynch {
    pub use super::*;

    pub use supply::{uncurry_trait_forwarding_info_for_AsyncI2c as uncurry_trait_forwarding_info_for_I2c, AsyncI2c as I2c};
}

/// I2c Configuration
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Config {
    pub frequency: fugit::HertzU32,
    pub timeout: Option<u32>,
}
impl Default for Config {
    fn default() -> Self {
        use fugit::RateExtU32;

        Self { frequency: 100.kHz(), timeout: Some(10) }
    }
}

mod alias {
    pub use embedded_hal::i2c::{ErrorType as I2cErrorType, I2c};
    pub use embedded_hal_async::i2c::I2c as AsyncI2c;
}

mod supply {
    use super::alias;
    use forward_traits::supply_forwarding_info_for_trait;

    supply_forwarding_info_for_trait! {
        alias::I2cErrorType,
        pub trait {
            type Error;
        }
    }

    supply_forwarding_info_for_trait! {
        alias::I2c,
        pub trait <A: ::varuemb::cross::i2c::AddressMode = ::varuemb::cross::i2c::SevenBitAddress> {
            #[inline]
            fn read(&mut self, address: A, read: &mut [::core::primitive::u8])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            fn write(&mut self, address: A, write: &[::core::primitive::u8])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            fn write_read(
                &mut self,
                address: A,
                write: &[::core::primitive::u8],
                read: &mut [::core::primitive::u8],
            ) -> ::core::result::Result<(), Self::Error>;
            #[inline]
            fn transaction(
                &mut self,
                address: A,
                operations: &mut [::varuemb::cross::i2c::Operation<'_>],
            ) -> ::core::result::Result<(), Self::Error>;
        }
    }

    supply_forwarding_info_for_trait! {
        alias::AsyncI2c,
        pub trait <A: ::varuemb::cross::i2c::AddressMode = ::varuemb::cross::i2c::SevenBitAddress> {
            #[inline]
            async fn read(&mut self, address: A, read: &mut [::core::primitive::u8])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            async fn write(&mut self, address: A, write: &[::core::primitive::u8])
                -> ::core::result::Result<(), Self::Error>;
            #[inline]
            async fn write_read(
                &mut self,
                address: A,
                write: &[::core::primitive::u8],
                read: &mut [::core::primitive::u8],
            ) -> ::core::result::Result<(), Self::Error>;
            #[inline]
            async fn transaction(
                &mut self,
                address: A,
                operations: &mut [::varuemb::cross::i2c::Operation<'_>],
            ) -> ::core::result::Result<(), Self::Error>;
        }
    }
}
