#![no_std]
#![feature(cfg_version)]
#![feature(strict_provenance)]
#![cfg_attr(not(version("1.84")), feature(exposed_provenance))]

pub mod luqueue;
pub use luqueue::{Item as LUQueueItem, LUQueue};
