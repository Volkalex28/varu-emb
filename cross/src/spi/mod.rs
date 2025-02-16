use enum_as_inner::EnumAsInner;

pub use embedded_hal::spi::*;

pub mod bus;
pub mod device;

pub use supply::{
    uncurry_trait_forwarding_info_for_SpiErrorType as uncurry_trait_forwarding_info_for_ErrorType, SpiErrorType as ErrorType,
};

pub mod asynch {
    pub use super::*;
}

/// SPI modes
#[derive(EnumAsInner, Debug, Clone, Copy, Eq, PartialEq)]
pub enum Mode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
}

/// SPI Bit Order
#[derive(EnumAsInner, Debug, Clone, Copy, Eq, PartialEq)]
pub enum BitOrder {
    MSBFirst,
    LSBFirst,
}

/// SPI data mode
///
/// Single = 1 bit, 2 wires
/// Dual = 2 bit, 2 wires
/// Quad = 4 bit, 4 wires
#[derive(EnumAsInner, Debug, Clone, Copy, Eq, PartialEq)]
pub enum DataMode {
    Single,
    Dual,
    Quad,
}

/// Spi Configuration
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Config {
    pub frequency: fugit::HertzU32,
    pub mode: Mode,
    pub bit_order: BitOrder,
    pub data_mode: DataMode,
}
impl Default for Config {
    fn default() -> Self {
        use fugit::RateExtU32;

        Self { frequency: 1.MHz(), mode: Mode::Mode0, bit_order: BitOrder::MSBFirst, data_mode: DataMode::Single }
    }
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
