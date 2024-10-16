#![cfg_attr(not(feature = "std"), no_std)]
// features
#![feature(error_in_core)]
#![feature(trait_alias)]

use core::error::Error;
pub use forward_traits::*;
pub use proc::*;

pub mod i2c;
pub mod io;
pub mod spi;

pub type IError<ET> = <ET as ErrorType>::Error;
pub type IResult<T> = Result<T, IError<T>>;
pub type Hw<I, const ID: usize, P = I> = <<P as Platform>::Origin as PlatformHw<I, ID>>::Hw;

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

pub trait Marker {}

pub trait Interface<P> {
    type Marker: Marker;
    type Hardware: Sized;
}

#[forward_traits::forwardable]
pub trait Platform: Sized + ErrorType {
    // type Init;
    type Origin: Platform<Origin = Self::Origin, Error = Self::Error>;

    // fn init(init: Self::Init) -> Result<(), Self::Error>;
}

// pub trait CrossPlatform<I: Interface<Self::Origin>>: Platform {
//     fn take() -> Result<I::Hardware, IError<Self>>;
// }

pub trait PlatformHw<I, const ID: usize>: Platform {
    type Hw: 'static;
}
impl<I: Interface<P>, const ID: usize, P: PlatformHw<I::Marker, ID>> PlatformHw<I, ID> for P {
    type Hw = P::Hw;
}

pub trait Component<'hw, I: Marker, const ID: usize, T: Sized = Self>: Sized + ErrorType {
    type Config;
    type Platform: Platform<Origin: PlatformHw<I, ID, Error: From<Self::Error>>>;

    fn init(hw: &'hw mut Hw<I, ID, Self::Platform>, config: Self::Config) -> Result<T, IError<Self>>;
}

pub mod blocking {
    use super::*;
    use spi::{bus, device};

    pub trait I2c<'hw, I: Marker, const ID: usize> = Component<'hw, I, ID> + i2c::I2c<Error = IError<Self>>;

    pub trait SpiBus<'hw, I: Marker, const ID: usize> = Component<'hw, I, ID> + bus::SpiBus<Error = IError<Self>>;
    pub trait SpiDevice<'hw, I: Marker, const ID: usize> = Component<'hw, I, ID> + device::SpiDevice<Error = IError<Self>>;

    pub trait Read<'hw, I: Marker, const ID: usize> = Component<'hw, I, ID> + io::Read<Error = IError<Self>>;
    pub trait Write<'hw, I: Marker, const ID: usize> = Component<'hw, I, ID> + io::Write<Error = IError<Self>>;
    pub trait ReadWrite<'hw, I: Marker, const ID: usize> = Read<'hw, I, ID> + Write<'hw, I, ID>;
}

pub mod asynch {
    use super::*;
    use i2c::asynch as i2c;
    use io::asynch as io;
    use spi::bus::asynch as bus;
    use spi::device::asynch as device;

    pub trait I2c<'hw, I: Marker, const ID: usize> = Component<'hw, I, ID> + i2c::I2c<Error = IError<Self>>;

    pub trait SpiBus<'hw, I: Marker, const ID: usize> = Component<'hw, I, ID> + bus::SpiBus<Error = IError<Self>>;
    pub trait SpiDevice<'hw, I: Marker, const ID: usize> = Component<'hw, I, ID> + device::SpiDevice<Error = IError<Self>>;

    pub trait Read<'hw, I: Marker, const ID: usize> = Component<'hw, I, ID> + io::Read<Error = IError<Self>>;
    pub trait Write<'hw, I: Marker, const ID: usize> = Component<'hw, I, ID> + io::Write<Error = IError<Self>>;
    pub trait ReadWrite<'hw, I: Marker, const ID: usize> = Read<'hw, I, ID> + Write<'hw, I, ID>;
}
