#![no_std]
#![cfg_attr(feature = "alloc", feature(min_specialization))]
#![cfg_attr(feature = "alloc", feature(trusted_len))]

pub mod sender;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;
