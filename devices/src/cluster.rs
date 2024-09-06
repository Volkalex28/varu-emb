use crate::{register::*, *};
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

mod assert_msg {
    pub const COUNT: &'static str = "For Cluster count should be > 1";
}

pub struct Proxy<R: Register, M: Mode = Blocking>(usize, PhantomData<*mut (R, M)>);
impl<R: Read> Proxy<R> {
    #[inline]
    pub fn read<D: Device>(&mut self, device: &mut D) -> Result<R, D::Error> {
        let reg = Adapter::empty(R::ADDRESS.map(|address| address + self.0 as u8));
        device.read_reg(reg)
    }
}
impl<R: Write> Proxy<R> {
    #[inline]
    pub fn write<D: Device>(&mut self, device: &mut D, data: R) -> Result<(), D::Error> {
        let reg = Adapter::new(R::ADDRESS.map(|address| address + self.0 as u8), data);
        device.write_reg(reg)
    }
}
impl<R: ReadWrite> Proxy<R> {
    #[inline]
    pub fn update<D: Device>(&mut self, device: &mut D, f: impl FnOnce(R) -> R) -> Result<(), D::Error> {
        let reg = self.read(device)?;
        self.write(device, f(reg))
    }
}

pub struct Cluster<R: Register, const N: usize> {
    proxy: Proxy<R>,
}
impl<R: Register, const N: usize> Cluster<R, N> {
    pub fn new() -> Self
    where
        assert::AssertMsg<{ R::COUNT > 1 }, { assert_msg::COUNT }>: assert::IsTrue,
    {
        Self { proxy: Proxy(0, PhantomData) }
    }

    pub fn select(&mut self, index: usize) -> Option<&mut Proxy<R>> {
        (index < N).then(|| {
            self.proxy.0 = index;
            &mut self.proxy
        })
    }

    fn panic_by_index(index: usize) -> ! {
        panic!("Index {index} out of range: (0..{N})")
    }
}
impl<R: Register, const N: usize> Index<usize> for Cluster<R, N> {
    type Output = Proxy<R>;
    fn index(&self, index: usize) -> &Self::Output {
        if index >= N {
            Self::panic_by_index(index)
        }
        &self.proxy
    }
}
impl<R: Register, const N: usize> IndexMut<usize> for Cluster<R, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.select(index).unwrap_or_else(|| Self::panic_by_index(index))
    }
}

mod asynch {
    use super::*;
    use crate::asynch::Device;
    use core::mem;

    impl<R: Register, const N: usize> Cluster<R, N> {
        pub fn select_async(&mut self, index: usize) -> Option<&mut Proxy<R, Async>> {
            (index < N).then(|| {
                self.proxy.0 = index;
                unsafe { mem::transmute(&mut self.proxy) }
            })
        }
    }

    impl<R: Register> Proxy<R> {
        pub fn cast_async(self) -> Proxy<R, Async> {
            Proxy(self.0, PhantomData)
        }
    }

    impl<R: Read> Proxy<R, Async> {
        pub fn cast_blocking(self) -> Proxy<R> {
            Proxy(self.0, PhantomData)
        }

        #[inline]
        pub async fn read<D: Device>(&mut self, device: &mut D) -> Result<R, D::Error> {
            let reg = Adapter::empty(R::ADDRESS.map(|address| address + self.0 as u8));
            device.read_reg(reg).await
        }
    }
    impl<R: Write> Proxy<R, Async> {
        #[inline]
        pub async fn write<D: Device>(&mut self, device: &mut D, data: R) -> Result<(), D::Error> {
            let reg = Adapter::new(R::ADDRESS.map(|address| address + self.0 as u8), data);
            device.write_reg(reg).await
        }
    }
    impl<R: ReadWrite> Proxy<R, Async> {
        #[inline]
        pub async fn update<D: Device>(&mut self, device: &mut D, f: impl FnOnce(R) -> R) -> Result<(), D::Error> {
            let reg = self.read(device).await?;
            self.write(device, f(reg)).await
        }
    }
}
