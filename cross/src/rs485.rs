use crate::gpio::OutputPin;
use crate::io;
use core::convert::Infallible;

struct Dir<'hw, D: OutputPin<Error = Infallible>>(&'hw mut D);
impl<'hw, D: OutputPin<Error = Infallible>> Dir<'hw, D> {
    fn new(dir: &'hw mut D) -> Self {
        _ = dir.set_high();
        Self(dir)
    }
}
impl<'hw, D: OutputPin<Error = Infallible>> Drop for Dir<'hw, D> {
    fn drop(&mut self) {
        _ = self.0.set_low();
    }
}

#[forward_traits::forward_receiver]
pub struct Rs485<RW, D> {
    pub rw: RW,
    pub dir: D,
}

impl<RW: io::ErrorType, D> io::ErrorType for Rs485<RW, D> {
    type Error = RW::Error;
}

impl<RW: io::Read, D> io::Read for Rs485<RW, D> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.rw.read(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), embedded_io::ReadExactError<Self::Error>> {
        self.rw.read_exact(buf)
    }
}

impl<RW: io::asynch::Read, D> io::asynch::Read for Rs485<RW, D> {
    #[inline]
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.rw.read(buf).await
    }

    #[inline]
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), embedded_io::ReadExactError<Self::Error>> {
        self.rw.read_exact(buf).await
    }
}

impl<RW: io::Write, D: OutputPin<Error = Infallible>> io::Write for Rs485<RW, D> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let dir = Dir::new(&mut self.dir);
        let len = self.rw.write(buf)?;

        self.rw.flush()?;
        drop(dir);

        Ok(len)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<RW: io::asynch::Write, D: OutputPin<Error = Infallible>> io::asynch::Write for Rs485<RW, D> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let dir = Dir::new(&mut self.dir);
        let len = self.rw.write(buf).await?;

        self.rw.flush().await?;
        drop(dir);

        Ok(len)
    }
}

pub mod hw {
    pub struct Rs485<RW, D> {
        pub rw: RW,
        pub dir: D,
    }
}

/* impl<P, RW: private::Allow + io::ErrorType, O: gpio::OutputPin> crate::Peripheral<Rs485<RW, O>> for P
where
    RW: private::Allow,
    utils::assert::Assert<{ <RW as private::Allow>::ALLOW_IMPL }>: utils::assert::IsTrue,
    P: crate::Peripheral<RW> + crate::Peripheral<O, Config = gpio::Level>,
{
    type Hw = hw::Rs485<crate::Hw<P, RW>, crate::Hw<P, O>>;
    type Config  <P as crate::Peripheral<RW>>::Config;

    fn init(hw: Self::Hw, config: Self::Config) -> Result<Rs485<RW, O>, Self::Error> {
        Ok(Rs485 {
            rw: <P as crate::Peripheral<RW>>::init(hw.rw, config)?,
            dir: <P as crate::Peripheral<O>>::init(hw.dir, gpio::Level::Low)?,
        })
    }
} */
