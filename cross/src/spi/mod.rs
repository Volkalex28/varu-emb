pub use embedded_hal::spi::*;

pub mod bus;
pub mod device;

pub use supply::uncurry_trait_forwarding_info_for_SpiErrorType as uncurry_trait_forwarding_info_for_ErrorType;
pub use supply::SpiErrorType as ErrorType;

pub mod asynch {
    pub use super::*;
}

mod alias {
    pub use embedded_hal::spi::ErrorType as SpiErrorType;
}

mod supply {
    use super::alias;
    use forward_traits::supply_forwarding_info_for_trait;

    supply_forwarding_info_for_trait! {
        alias::SpiErrorType,
        pub trait {
            type Error;
        }
    }
}
