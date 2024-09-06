use crate::register::*;
use crate::{Async, Blocking, Mode};
use core::borrow::*;
use core::marker::PhantomData;
use embassy_time::block_for;
use embassy_time::Timer;
use embedded_hal::spi::{SpiBus, SpiDevice};
use macros::*;

pub struct Device<I, M: Mode = Blocking> {
    interface: I,
    _mode: PhantomData<M>,
}
impl<I, M: Mode> Borrow<I> for Device<I, M> {
    fn borrow(&self) -> &I {
        &self.interface
    }
}
impl<I, M: Mode> BorrowMut<I> for Device<I, M> {
    fn borrow_mut(&mut self) -> &mut I {
        &mut self.interface
    }
}
impl<I: SpiDevice> Device<I, Blocking> {
    pub fn new(spi: I) -> Self {
        Self { interface: spi, _mode: PhantomData }
    }

    implementation!();
}
impl<I: SpiDevice> crate::private::Device for Device<I, Blocking> {
    type Error = I::Error;

    #[inline(always)]
    fn read_reg<R: Read>(&mut self, reg: Adapter<R, false>) -> Result<R, Self::Error> {
        Device::<I, Blocking>::read(self, reg)
    }

    #[inline(always)]
    fn write_reg<R: Write>(&mut self, reg: Adapter<R, true>) -> Result<(), Self::Error> {
        Device::<I, Blocking>::write(self, reg)
    }
}

pub struct Bus<I, M: Mode = Blocking> {
    interface: I,
    _mode: PhantomData<M>,
}
impl<I, M: Mode> Borrow<I> for Bus<I, M> {
    fn borrow(&self) -> &I {
        &self.interface
    }
}
impl<I, M: Mode> BorrowMut<I> for Bus<I, M> {
    fn borrow_mut(&mut self) -> &mut I {
        &mut self.interface
    }
}
impl<I: SpiBus> Bus<I, Blocking> {
    pub fn new(spi: I) -> Self {
        Self { interface: spi, _mode: PhantomData }
    }

    implementation!();
}
impl<I: SpiBus> crate::private::Device for Bus<I, Blocking> {
    type Error = I::Error;

    #[inline(always)]
    fn read_reg<R: Read>(&mut self, reg: Adapter<R, false>) -> Result<R, Self::Error> {
        Bus::<I, Blocking>::read(self, reg)
    }

    #[inline(always)]
    fn write_reg<R: Write>(&mut self, reg: Adapter<R, true>) -> Result<(), Self::Error> {
        Bus::<I, Blocking>::write(self, reg)
    }
}

mod asynch {
    use super::*;
    use embedded_hal_async::spi::{SpiBus, SpiDevice};

    impl<I: SpiDevice> Device<I, Async> {
        pub fn new_async(spi: I) -> Self {
            Self { interface: spi, _mode: PhantomData }
        }

        implementation!(async .await);
    }
    impl<I: SpiDevice> crate::private::asynch::Device for Device<I, Async> {
        type Error = I::Error;

        #[inline(always)]
        async fn read_reg<R: Read>(&mut self, reg: Adapter<R, false>) -> Result<R, Self::Error> {
            Device::<I, Async>::read(self, reg).await
        }

        #[inline(always)]
        async fn write_reg<R: Write>(&mut self, reg: Adapter<R, true>) -> Result<(), Self::Error> {
            Device::<I, Async>::write(self, reg).await
        }
    }

    impl<I: SpiBus> Bus<I, Async> {
        pub fn new_async(spi: I) -> Self {
            Self { interface: spi, _mode: PhantomData }
        }

        implementation!(async .await);
    }
    impl<I: SpiBus> crate::private::asynch::Device for Bus<I, Async> {
        type Error = I::Error;

        #[inline(always)]
        async fn read_reg<R: Read>(&mut self, reg: Adapter<R, false>) -> Result<R, Self::Error> {
            Bus::<I, Async>::read(self, reg).await
        }

        #[inline(always)]
        async fn write_reg<R: Write>(&mut self, reg: Adapter<R, true>) -> Result<(), Self::Error> {
            Bus::<I, Async>::write(self, reg).await
        }
    }
}

mod macros {
    macro_rules! timeout {
        ($t:expr => ) => {
            block_for($t)
        };
        ($t:expr => $($a:tt)+) => {
            Timer::after($t).await
        };
    }

    macro_rules! implementation {
        ($($f:tt $($a:tt)+)?) => {
            pub fn into_raw(self) -> I {
                self.interface
            }

            $($f)? fn write<R: Write>(&mut self, mut reg: Adapter<R, true>) -> Result<(), I::Error> {
                let (_, data) = reg.prepare();

                self.interface.write(data) $($($a)+)? ?;

                if R::TIMEOUT.as_millis() > 0 {
                    timeout!(R::TIMEOUT => $($($a)+)?)
                }

                Ok(())
            }

            $($f)? fn read<R: Read>(&mut self, mut reg: Adapter<R, false>) -> Result<R, I::Error> {
                let (addr, data) = reg.prepare(true);

                if let Some(address) = addr.filter(|_| R::TIMEOUT.as_millis() == 0) {
                    self.read_with_address_no_timeout::<R>(address, data) $($($a)+)? ?;
                } else {
                    let (addr, data) = data.split_at_mut(addr.is_some() as usize);
                    if !addr.is_empty() {
                        self.interface.write(addr) $($($a)+)? ?;
                    }
                    timeout!(R::TIMEOUT => $($($a)+)?);

                    self.interface.read(data) $($($a)+)? ?;
                }


                Ok(reg.finish())
            }

            $($f)? fn read_with_address_no_timeout<R: Register>(&mut self, address: u8, data: &mut [u8]) -> Result<(), I::Error> {
                let write = [address];
                self.interface.transfer(data, &write) $($($a)+)? ?;

                Ok(())
            }
        };
    }

    pub(super) use implementation;
    pub(super) use timeout;
}
