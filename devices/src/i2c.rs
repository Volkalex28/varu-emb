use self::macros::*;
use self::HalOperation::*;
use self::Operation::*;
use super::{Async, Blocking, Mode};
use crate::register::{self, *};
use core::borrow::*;
use embassy_time::{block_for, Duration, Timer};
use embedded_hal::i2c::{self, I2c, Operation as HalOperation};

#[derive(Debug)]
pub enum Operation<'a> {
    Timeout(Option<Duration>),
    Operation(HalOperation<'a>),
}

pub struct Device<I, M: Mode = Blocking, A: i2c::AddressMode = i2c::SevenBitAddress> {
    address: A,
    interface: I,
    _mode: M,
}
impl<I, M: Mode, A: i2c::AddressMode> Borrow<I> for Device<I, M, A> {
    fn borrow(&self) -> &I {
        &self.interface
    }
}
impl<I, M: Mode, A: i2c::AddressMode> BorrowMut<I> for Device<I, M, A> {
    fn borrow_mut(&mut self) -> &mut I {
        &mut self.interface
    }
}
impl<I: I2c<A>, A: i2c::AddressMode + Copy> Device<I, Blocking, A> {
    pub fn new(interface: I, address: A) -> Self {
        Self { address, interface, _mode: Blocking }
    }

    implementation!();
}
impl<I: I2c<A>, A: i2c::AddressMode + Copy> crate::private::Device for Device<I, Blocking, A> {
    type Error = I::Error;

    #[inline(always)]
    fn read_reg<R: register::Read>(&mut self, reg: Adapter<R, false>) -> Result<R, Self::Error> {
        Device::<I, Blocking, A>::read(self, reg)
    }

    #[inline(always)]
    fn write_reg<R: register::Write>(&mut self, reg: Adapter<R, true>) -> Result<(), Self::Error> {
        Device::<I, Blocking, A>::write(self, reg)
    }
}

pub mod asynch {
    use super::*;
    use embedded_hal_async::i2c::{self, I2c};

    impl<I: I2c<A>, A: i2c::AddressMode + Copy> Device<I, Async, A> {
        pub fn new_async(interface: I, address: A) -> Self {
            Self { address, interface, _mode: Async }
        }

        implementation!(async .await);
    }
    impl<I: I2c<A>, A: i2c::AddressMode + Copy> crate::private::asynch::Device for Device<I, Async, A> {
        type Error = I::Error;

        #[inline(always)]
        async fn read_reg<R: register::Read>(&mut self, reg: Adapter<R, false>) -> Result<R, Self::Error> {
            Device::<I, Async, A>::read(self, reg).await
        }

        #[inline(always)]
        async fn write_reg<R: register::Write>(&mut self, reg: Adapter<R, true>) -> Result<(), Self::Error> {
            Device::<I, Async, A>::write(self, reg).await
        }
    }
}

mod macros {
    macro_rules! timeout {
        ($c:ident => ) => {
            block_for(*$c)
        };
        ($c:ident => $($a:tt)+) => {
            Timer::after(*$c).await
        };
    }

    macro_rules! implementation {
        ($($f:tt $($a:tt)+)?) => {
            pub $($f)? fn address(&self) -> A {
                self.address
            }

            pub fn into_raw(self) -> I {
                self.interface
            }

            $($f)? fn read<R: register::Read>(&mut self, mut reg: Adapter<R, false>) -> Result<R, I::Error>
            {
                let (addr, data) = reg.prepare(false);
                self.read_impl(addr, R::TIMEOUT, data) $($($a)+)? ?;

                Ok(reg.finish())
            }

            $($f)? fn write<R: register::Write>(&mut self, mut reg: Adapter<R, true>) -> Result<(), I::Error>
            {
                let (_, data) = reg.prepare();
                self.write_impl(R::TIMEOUT, &data) $($($a)+)?
            }

            $($f)? fn read_impl(&mut self, address: Option<u8>, timeout: Duration, data: &mut [u8]) -> Result<(), I::Error> {
                if data.is_empty() && address.is_none() {
                    return Ok(());
                }

                let data_is_empty = data.is_empty();
                let address = address.as_slice();
                let timeout = (timeout.as_millis() > 0).then_some(timeout);

                let mut ops = [Operation(Write(address)), Timeout(timeout), Operation(Read(data))];

                let mut ops = ops.as_mut_slice();
                if data_is_empty {
                    ops = &mut ops[..2];
                }
                if address.is_empty() {
                    ops = &mut ops[2..];
                }

                self.transaction(ops) $($($a)+)?
            }

            $($f)? fn write_impl(&mut self, timeout: Duration, data: &[u8]) -> Result<(), I::Error> {
                if data.is_empty() {
                    return Ok(());
                }

                let timeout = (timeout.as_millis() > 0).then_some(timeout);

                let mut ops = [Operation(Write(data)), Timeout(timeout)];
                self.transaction(&mut ops) $($($a)+)?
            }

            $($f)? fn transaction(&mut self, operations: &mut [Operation<'_>]) -> Result<(), I::Error> {
                let [Operation(Write(bytes)), Timeout(None), Operation(Read(buffer))] = operations else {
                    for operation in operations {
                        match operation {
                            Timeout(Some(dur)) => timeout!(dur => $($($a)+)?),
                            Operation(Read(read)) => {
                                self.interface.read(self.address, read) $($($a)+)? ?;
                            }
                            Operation(Write(write)) => {
                                self.interface.write(self.address, write) $($($a)+)? ?;
                            }
                            Timeout(None) => continue,
                        }
                    }
                    return Ok(());
                };
                self.interface.write_read(self.address, bytes, buffer) $($($a)+)?
            }
        };
    }

    pub(super) use {implementation, timeout};
}
