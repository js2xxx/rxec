#![no_std]

#[cfg(feature = "alloc")]
mod future;
mod traits;
pub mod tuple_list;

#[cfg(feature = "alloc")]
extern crate alloc;

pub use self::{
    future::Cps,
    traits::{Execution, Receiver, ReceiverFrom, Sender, SenderTo},
};
