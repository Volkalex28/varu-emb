#![no_std]
// Features
#![feature(generic_const_exprs)]
#![feature(adt_const_params)]
#![feature(auto_traits, negative_impls)]
// Warnings disable
#![cfg_attr(test, allow(dead_code))]
#![allow(async_fn_in_trait)]
#![allow(incomplete_features)]

#[macro_use]
#[cfg(test)]
extern crate std;

use utils::assert;

pub use cluster::*;
pub use endian_trait::*;
pub use register::*;

pub mod cluster;
pub mod i2c;
pub mod register;
pub mod spi;

pub trait Mode: private::Sealed {}

pub struct Async;
impl private::Sealed for Async {}
impl Mode for Async {}

pub struct Blocking;
impl private::Sealed for Blocking {}
impl Mode for Blocking {}

pub trait Device: private::Device {
    #[inline]
    fn read<R: ReadOne>(&mut self) -> Result<R, Self::Error> {
        self.read_reg(Adapter::empty(R::ADDRESS))
    }
    #[inline]
    fn write<R: WriteOne>(&mut self, reg: R) -> Result<(), Self::Error> {
        self.write_reg(Adapter::new(R::ADDRESS, reg))
    }
    #[inline]
    fn update<R: ReadWriteOne>(&mut self, f: impl FnOnce(R) -> R) -> Result<(), Self::Error> {
        let reg = self.read()?;
        self.write(f(reg))
    }
}
impl<D: private::Device> Device for D {}

pub mod asynch {
    use super::*;

    pub trait Device: private::asynch::Device {
        #[inline]
        async fn read<R: ReadOne>(&mut self) -> Result<R, Self::Error> {
            self.read_reg(Adapter::empty(R::ADDRESS)).await
        }
        #[inline]
        async fn write<R: WriteOne>(&mut self, reg: R) -> Result<(), Self::Error> {
            self.write_reg(Adapter::new(R::ADDRESS, reg)).await
        }

        async fn update<R: ReadWriteOne>(
            &mut self,
            f: impl FnOnce(R) -> R,
        ) -> Result<(), Self::Error> {
            let reg = self.read().await?;
            self.write(f(reg)).await
        }
    }
    impl<D: private::asynch::Device> Device for D {}
}

mod private {
    use super::*;

    pub trait Sealed {}

    pub trait Device {
        type Error: core::fmt::Debug;

        fn read_reg<R: Read>(&mut self, reg: Adapter<R, false>) -> Result<R, Self::Error>;
        fn write_reg<R: Write>(&mut self, reg: Adapter<R, true>) -> Result<(), Self::Error>;
    }

    pub mod asynch {
        use super::*;

        pub trait Device {
            type Error: core::fmt::Debug;

            async fn read_reg<R: Read>(&mut self, reg: Adapter<R, false>)
                -> Result<R, Self::Error>;
            async fn write_reg<R: Write>(
                &mut self,
                reg: Adapter<R, true>,
            ) -> Result<(), Self::Error>;
        }
    }
}
