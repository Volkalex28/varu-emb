impl<'a> TryFrom<Newtype<&'a [u8]>> for &'a str {
    type Error = ();
    fn try_from(Newtype(cstr): Newtype<&'a [u8]>) -> Result<Self, Self::Error> {
        let nul = cstr.iter().position(|x| *x == 0).ok_or(())?;
        core::str::from_utf8(&cstr[..nul]).map_err(|_| ())
    }
}

impl<'a, const LEN: usize> TryFrom<Newtype<&'a [u8; LEN]>> for &'a str {
    type Error = ();
    fn try_from(Newtype(cstr): Newtype<&'a [u8; LEN]>) -> Result<Self, Self::Error> {
        Newtype(cstr as &[u8]).try_into()
    }
}

impl<'a> TryFrom<Newtype<&'a [core::ffi::c_char]>> for &'a str {
    type Error = ();
    fn try_from(Newtype(cstr): Newtype<&'a [core::ffi::c_char]>) -> Result<Self, Self::Error> {
        Newtype(bytemuck::cast_slice::<_, u8>(cstr)).try_into()
    }
}

impl<'a, const LEN: usize> TryFrom<Newtype<&'a [core::ffi::c_char; LEN]>> for &'a str {
    type Error = ();
    fn try_from(Newtype(cstr): Newtype<&'a [core::ffi::c_char; LEN]>) -> Result<Self, Self::Error> {
        Newtype(cstr as &[core::ffi::c_char]).try_into()
    }
}

pub struct Newtype<T>(pub T);
pub trait NewtypeFrom<T>: From<Newtype<T>> + Sized {
    fn from_nt(data: T) -> Self {
        Newtype(data).into()
    }
}
impl<T, S: From<Newtype<T>>> NewtypeFrom<T> for S {}
pub trait NewtypeInto<T: From<Newtype<Self>>>: Sized {
    fn into_nt(self) -> T {
        Newtype(self).into()
    }
}
impl<T: From<Newtype<Self>>, S> NewtypeInto<T> for S {}
pub trait NewtypeTryFrom<T>: TryFrom<Newtype<T>> + Sized {
    fn try_from_nt(data: T) -> Result<Self, Self::Error> {
        Newtype(data).try_into()
    }
}
impl<T, S: TryFrom<Newtype<T>>> NewtypeTryFrom<T> for S {}
pub trait NewtypeTryInto<T: TryFrom<Newtype<Self>>>: Sized {
    fn try_into_nt(self) -> Result<T, T::Error> {
        Newtype(self).try_into()
    }
}
impl<T: TryFrom<Newtype<Self>>, S> NewtypeTryInto<T> for S {}
