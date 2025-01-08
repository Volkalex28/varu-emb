use enum_as_derive::EnumAs;

pub use embedded_hal::digital::*;
pub use supply::{GpioErrorType as ErrorType, InputPin, OutputPin, StatefulOutputPin};

pub use supply::{
    uncurry_trait_forwarding_info_for_GpioErrorType as uncurry_trait_forwarding_info_for_ErrorType,
    uncurry_trait_forwarding_info_for_InputPin, uncurry_trait_forwarding_info_for_OutputPin,
    uncurry_trait_forwarding_info_for_StatefulOutputPin,
};

pub mod asynch {
    pub use super::*;

    pub use supply::{
        uncurry_trait_forwarding_info_for_AsyncInputPin as uncurry_trait_forwarding_info_for_InputPin,
        AsyncInputPin as InputPin,
    };
}

/// Pull setting for an input.
#[derive(EnumAs, Debug, Eq, PartialEq, Copy, Clone)]
pub enum Pull {
    /// No pull
    None,
    /// Pull up
    Up,
    /// Pull down
    Down,
}
impl From<Option<bool>> for Pull {
    fn from(val: Option<bool>) -> Self {
        match val {
            Some(true) => Self::Up,
            Some(false) => Self::Down,
            None => Self::None,
        }
    }
}

/// Digital input or output level.
#[derive(EnumAs, Debug, Eq, PartialEq, Copy, Clone)]
pub enum Level {
    /// Low
    Low,
    /// High
    High,
}
impl From<bool> for Level {
    fn from(val: bool) -> Self {
        match val {
            true => Self::High,
            false => Self::Low,
        }
    }
}

/// GPIO Configuration
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Config {
    pub pull: Pull,
    pub initial_level: Level,
}

mod alias {
    pub use embedded_hal::digital::{ErrorType as GpioErrorType, InputPin, OutputPin, StatefulOutputPin};
    pub use embedded_hal_async::digital::Wait as AsyncInputPin;
}

mod supply {
    use super::alias;
    use forward_traits::supply_forwarding_info_for_trait;

    supply_forwarding_info_for_trait! {
        alias::GpioErrorType,
        pub trait {
            type Error;
        }
    }

    supply_forwarding_info_for_trait! {
        alias::InputPin,
        pub trait {
            #[inline]
            fn is_high(&mut self) -> ::core::result::Result<::core::primitive::bool, Self::Error>;

            #[inline]
            fn is_low(&mut self) -> ::core::result::Result<::core::primitive::bool, Self::Error>;
        }
    }

    supply_forwarding_info_for_trait! {
        alias::OutputPin,
        pub trait {
            #[inline]
            fn set_low(&mut self) -> ::core::result::Result<(), Self::Error>;

            #[inline]
            fn set_high(&mut self) -> ::core::result::Result<(), Self::Error>;

            #[inline]
            fn set_state(&mut self, state: ::varuemb::cross::gpio::PinState) -> ::core::result::Result<(), Self::Error>;
        }
    }

    supply_forwarding_info_for_trait! {
        alias::StatefulOutputPin,
        pub trait {
            #[inline]
            fn is_set_high(&mut self) -> ::core::result::Result<::core::primitive::bool, Self::Error>;

            #[inline]
            fn is_set_low(&mut self) -> ::core::result::Result<::core::primitive::bool, Self::Error>;

            #[inline]
            fn toggle(&mut self) -> ::core::result::Result<(), Self::Error>;
        }
    }

    supply_forwarding_info_for_trait! {
        alias::AsyncInputPin,
        pub trait {
            #[inline]
            async fn wait_for_high(&mut self) -> ::core::result::Result<(), Self::Error>;

            #[inline]
            async fn wait_for_low(&mut self) -> ::core::result::Result<(), Self::Error>;

            #[inline]
            async fn wait_for_rising_edge(&mut self) -> ::core::result::Result<(), Self::Error>;

            #[inline]
            async fn wait_for_falling_edge(&mut self) -> ::core::result::Result<(), Self::Error>;

            #[inline]
            async fn wait_for_any_edge(&mut self) -> ::core::result::Result<(), Self::Error>;
        }
    }
}
