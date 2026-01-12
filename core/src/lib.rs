#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(trait_alias)]

extern crate alloc;

#[cfg(test)]
extern crate std;

mod list;
mod traits;
pub use self::traits::{OperationState, Receiver, ReceiverFrom, Scheduler, Sender, SenderTo};

mod basic;
pub mod util;
