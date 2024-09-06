#![no_std]
#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(associated_type_defaults)]
#![feature(const_closures)]
#![feature(const_maybe_uninit_array_assume_init)]
#![feature(const_maybe_uninit_uninit_array)]
#![feature(const_maybe_uninit_write)]
#![feature(const_mut_refs)]
#![feature(const_refs_to_cell)]
#![feature(const_trait_impl)]
#![feature(generic_const_exprs)]
#![feature(macro_metavar_expr)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(maybe_uninit_uninit_array)]

pub mod array_init;
pub mod assert;
pub mod futures;
pub mod macros;
pub mod newtype;

pub use array_init::ArrayInitializer;
// pub use newtype::*;
// pub use varuemb_utils_proc::multi_impl_block;

pub mod __private {
    pub use const_format;
    pub use embassy_futures;
    pub use paste;
}
