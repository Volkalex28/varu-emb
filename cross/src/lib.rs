#![cfg_attr(not(feature = "std"), no_std)]
// features
#![feature(trait_alias)]

use core::error::Error;
pub use forward_traits::*;
pub use proc::*;

pub mod gpio;
pub mod i2c;
pub mod io;
pub mod rs485;
pub mod spi;

pub type IError<ET> = <ET as ErrorType>::Error;
pub type IResult<T> = Result<T, IError<T>>;
pub type Hw<P, T> = <P as Peripheral<T>>::Hw;

#[forward_traits::forwardable]
pub trait ErrorType {
    type Error: Error;
}

impl<T: ErrorType + ?Sized> ErrorType for &T {
    type Error = T::Error;
}

impl<T: ErrorType + ?Sized> ErrorType for &mut T {
    type Error = T::Error;
}

#[forward_traits::forwardable]
pub trait Platform: Sized + ErrorType {}

pub trait Interface<P: Platform> {
    type Hardware;

    fn init<T: Sized>(hw: P::Hw, config: P::Config) -> Result<T, IError<P>>
    where
        P: Peripheral<T>,
    {
        <P as Peripheral<T>>::init(hw, config)
    }
}

#[forward_traits::forwardable]
pub trait Peripherals<I: Interface<Self>>: Platform {
    type Config;

    fn take(config: Self::Config) -> Result<I::Hardware, Self::Error>;
}

#[forward_traits::forwardable]
pub trait Peripheral<T: Sized>: Platform {
    type Hw;
    type Config;

    fn init(hw: Self::Hw, config: Self::Config) -> Result<T, Self::Error>;
}

pub mod blocking {
    pub use crate::gpio::{self, InputPin, OutputPin, StatefulOutputPin};

    pub use crate::i2c::{self, I2c};

    pub use crate::io::{self as io, Read, Write};
    pub trait ReadWrite = Read + Write;

    pub use crate::spi::bus::{self as spi_bus, SpiBus};
    pub use crate::spi::device::{self as spi_device, SpiDevice};
}

pub mod asynch {
    pub use crate::gpio::asynch::{self as gpio, InputPin};

    pub use crate::i2c::asynch::{self as i2c, I2c};

    pub use crate::io::asynch::{self as io, Read, Write};
    pub trait ReadWrite = Read + Write;

    pub use crate::spi::bus::asynch::{self as spi_bus, SpiBus};
    pub use crate::spi::device::asynch::{self as spi_device, SpiDevice};
}
