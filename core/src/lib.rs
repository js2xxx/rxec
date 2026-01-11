#![no_std]

#[cfg(feature = "alloc")]
mod future;
mod traits;
pub mod tuple_list;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub use self::future::Cps;
pub use self::traits::{Execution, Receiver, ReceiverFrom, Sender, SenderTo};
