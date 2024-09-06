#![no_std]
#![feature(exposed_provenance)]

pub mod luqueue;
pub use luqueue::{Item as LUQueueItem, LUQueue};
