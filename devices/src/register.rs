use crate::assert::*;
use core::mem::MaybeUninit;
use endian_trait::Endian;

pub(crate) use private::Adapter;

pub use embassy_time::Duration;
pub use proc::Register;

mod assert_msg {
    pub const READ_SIZE: &'static str = "Read size > register data size";
    pub const WRITE_SIZE: &'static str = "Write size > register data size";
    pub const COUNT: &'static str = "Register count > 1. Use Cluster";
    pub const COUNT_EMPTY: &'static str = "Register count == 0";
}

#[derive(Debug, Clone, Copy)]
pub enum Order {
    Msb,
    Lsb,
}

pub trait Instance: Endian + Sized {
    const ORDER: Order;
    const COUNT: usize;
}

pub trait Config<const WRITE: bool>: Instance {
    const ADDRESS: Option<u8>;
    const SIZE: usize;
    const TIMEOUT: embassy_time::Duration;
}

pub trait Register: Instance + private::Valid {}
impl<R: Instance + private::Valid> Register for R {}

pub trait Read: Register + Config<false> + private::ValidRead {}
impl<R: Register + Config<false> + private::ValidRead> Read for R {}

pub trait ReadOne: Read {}
impl<R: Read> ReadOne for R where AssertMsg<{ R::COUNT == 1 }, { assert_msg::COUNT }>: IsTrue {}

pub trait Write: Register + Config<true> + private::ValidWrite {}
impl<R: Register + Config<true> + private::ValidWrite> Write for R {}

pub trait WriteOne: Write {}
impl<R: Write> WriteOne for R where AssertMsg<{ R::COUNT == 1 }, { assert_msg::COUNT }>: IsTrue {}

pub trait ReadWrite: Read + Write {}
impl<R: Read + Write> ReadWrite for R {}

pub trait ReadWriteOne: ReadOne + WriteOne {}
impl<R: ReadOne + WriteOne> ReadWriteOne for R where AssertMsg<{ R::COUNT == 1 }, { assert_msg::COUNT }>: IsTrue {}

pub(crate) mod private {
    use super::*;
    use core::{mem, slice};

    pub auto trait Valid {}
    impl<R, const W: bool> !Valid for Adapter<R, W> {}
    impl<R: Instance> Valid for R where AssertMsg<{ R::COUNT > 0 }, { assert_msg::COUNT_EMPTY }>: IsTrue {}

    pub trait ValidRead: Valid {}
    impl<R: Register + Config<false>> ValidRead for R where
        AssertMsg<{ R::SIZE <= mem::size_of::<R>() }, { assert_msg::READ_SIZE }>: IsTrue
    {
    }

    pub trait ValidWrite: Valid {}
    impl<R: Register + Config<true>> ValidWrite for R where
        AssertMsg<{ R::SIZE <= mem::size_of::<R>() }, { assert_msg::WRITE_SIZE }>: IsTrue
    {
    }

    struct Erased {
        data: *mut u8,
        size: usize,
        size_of: usize,
        order: Order,
        address: Option<u8>,
    }
    impl Erased {
        fn new<R: Register>(reg: &mut MaybeUninit<R>, size: usize, address: Option<u8>) -> Self {
            Self { data: reg.as_mut_ptr().cast(), size, size_of: mem::size_of::<R>(), order: R::ORDER, address }
        }

        unsafe fn make_slice<'a>(mut data: *mut u8, offset: isize, len: usize) -> &'a mut [u8] {
            unsafe {
                data = data.offset(offset);
                slice::from_raw_parts_mut(data, len)
            }
        }

        fn reg_to_bytes<'a>(&self, offset: isize) -> &'a mut [u8] {
            let data_offset = if matches!(self.order, Order::Msb) { self.size_of - self.size } else { 0 };
            let len = self.size as isize - offset;
            unsafe { Self::make_slice(self.data, data_offset as isize + offset, len as usize) }
        }

        fn prepare<'a>(&self) -> &'a mut [u8] {
            let data;

            if let Some(address) = self.address {
                data = self.reg_to_bytes(-1);
                data[0] = address;
            } else {
                data = self.reg_to_bytes(0);
            }

            data
        }
    }

    #[repr(C, align(1))]
    pub struct Adapter<R, const ALLOW_WRITE: bool> {
        address: Option<u8>,
        data: MaybeUninit<R>,
    }
    impl<R: Register, const ALLOW_WRITE: bool> Adapter<R, ALLOW_WRITE> {
        fn apply_reg_order(data: R) -> R {
            match R::ORDER {
                Order::Msb => data.to_be(),
                Order::Lsb => data.to_le(),
            }
        }
    }
    impl<R: Write> Adapter<R, true> {
        pub fn new(address: Option<u8>, data: R) -> Self {
            Self { address, data: MaybeUninit::new(data) }
        }

        pub fn prepare(&mut self) -> (Option<u8>, &[u8]) {
            unsafe { self.data.write(Self::apply_reg_order(self.data.assume_init_read())) };

            let reg = Erased::new(&mut self.data, R::SIZE, self.address);
            (self.address, reg.prepare())
        }
    }
    impl<R: Read> Adapter<R, false> {
        pub fn empty(address: Option<u8>) -> Self {
            Self { address, data: MaybeUninit::zeroed() }
        }

        pub(crate) fn prepare(&mut self, data_with_address: bool) -> (Option<u8>, &mut [u8]) {
            let reg = Erased::new(&mut self.data, R::SIZE, self.address.filter(|_| data_with_address));
            (self.address, reg.prepare())
        }

        pub fn finish(self) -> R {
            Self::apply_reg_order(unsafe { self.data.assume_init() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embassy_time::Duration;

    proc_bitfield::bitfield! {
        #[derive(Endian, Clone, Copy)]
        struct Msb(u32): Debug {
            data0: u8 @ 0..=7,
            data1: u8 @ 8..=15,
            data2: u8 @ 16..=23,
            // data3: u8 @ 24..=31,
        }
    }
    impl Instance for Msb {
        const ORDER: Order = Order::Msb;
        const COUNT: usize = 1;
    }
    impl Config<false> for Msb {
        const ADDRESS: Option<u8> = Some(20);
        const SIZE: usize = 2;
        const TIMEOUT: embassy_time::Duration = Duration::from_millis(0);
    }
    impl Config<true> for Msb {
        const ADDRESS: Option<u8> = Some(20);
        const SIZE: usize = 2;
        const TIMEOUT: embassy_time::Duration = Duration::from_millis(0);
    }

    #[test]
    fn test_to_write_msb() {
        let mut reg = Adapter::new(<Msb as Config<true>>::ADDRESS, Msb(0x12345678));
        let (addr, data) = reg.prepare();

        assert_eq!(Some(20), addr);

        assert_eq!(data, [20, 0x56, 0x78]);
        assert_eq!(&data[1..], &0x12345678u32.to_be_bytes()[2..]);
    }

    #[test]
    fn test_from_read_msb() {
        let data = [0x56, 0x78];
        let reg: Msb = {
            let mut reg = Adapter::empty(<Msb as Config<false>>::ADDRESS);
            reg.prepare(false).1.copy_from_slice(&data);
            reg.finish()
        };

        assert_eq!(reg.0, 0x00005678);
        assert_eq!(reg.0, u32::from_be_bytes([0, 0, 0x56, 0x78]));
    }

    proc_bitfield::bitfield! {
        #[derive(Endian, Clone, Copy)]
        struct Lsb(u32): Debug {
            data0: u8 @ 0..=7,
            data1: u8 @ 8..=15,
            data2: u8 @ 16..=23,
            // data3: u8 @ 24..=31,
        }
    }
    impl Instance for Lsb {
        const ORDER: Order = Order::Lsb;
        const COUNT: usize = 1;
    }
    impl Config<false> for Lsb {
        const ADDRESS: Option<u8> = Some(20);
        const SIZE: usize = 2;
        const TIMEOUT: embassy_time::Duration = Duration::from_millis(0);
    }
    impl Config<true> for Lsb {
        const ADDRESS: Option<u8> = Some(21);
        const SIZE: usize = 3;
        const TIMEOUT: embassy_time::Duration = Duration::from_millis(0);
    }

    #[test]
    fn test_to_write_lsb() {
        let mut reg = Adapter::new(<Lsb as Config<true>>::ADDRESS, Lsb(0x12345678));
        let (addr, data) = reg.prepare();

        assert_eq!(Some(21), addr);

        assert_eq!(data, [21, 0x78, 0x56, 0x34]);
        assert_eq!(&data[1..], &0x12345678u32.to_le_bytes()[..3]);
    }

    #[test]
    fn test_from_read_lsb() {
        let data = [0x78, 0x56];
        let reg: Lsb = {
            let mut reg = Adapter::empty(<Lsb as Config<false>>::ADDRESS);
            let (addr, read) = reg.prepare(true);

            assert_eq!(20, read[0]);
            assert_eq!(Some(20), addr);

            (&mut read[1..]).copy_from_slice(&data);
            reg.finish()
        };

        assert_eq!(reg.0, 0x00005678);
        assert_eq!(reg.0, u32::from_le_bytes([0x78, 0x56, 0, 0]));
    }
}
