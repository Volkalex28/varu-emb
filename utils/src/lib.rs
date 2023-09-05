#![allow(incomplete_features)]
#![feature(const_mut_refs)]
#![feature(const_closures)]
#![feature(const_trait_impl)]
#![feature(const_refs_to_cell)]
#![feature(macro_metavar_expr)]
#![feature(const_maybe_uninit_write)]
#![feature(maybe_uninit_uninit_array_transpose)]

pub mod array_init;
pub mod execution;
pub mod macros;
pub mod newtype;
pub mod proc_meta_parser;

use std::marker::PhantomData;

pub use array_const_fn_init::array_const_fn_init;
pub use array_init::ArrayInitializer;
pub use newtype::*;
pub use varuemb_utils_proc::multi_impl_block;

pub mod __private {
    pub use const_format;
    pub use embassy_futures;
    pub use paste;
}


#[const_trait]
pub trait ConstDefault: Sized {
    fn default() -> Self;
}

impl<T: ?Sized> const ConstDefault for PhantomData<T> {
    fn default() -> Self {
        Self
    }
}