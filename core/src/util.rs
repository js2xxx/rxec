mod and_then;
mod future;
mod map;
mod value;
mod wait;

#[cfg(feature = "std")]
pub use self::wait::sync_wait;
pub use self::{
    and_then::{AndThen, and_then},
    future::{Async, async_},
    map::{Map, map},
    value::{Value, value},
    wait::{CanceledError, wait},
};

const ONESHOT_COMPLETED: &str = "oneshot sender already completed";

#[cfg(test)]
mod tests {
    use placid::pown;

    use super::*;
    use crate::{OperationState, Receiver, SenderTo};

    struct DummyReceiver;

    impl<T: core::fmt::Debug> Receiver<T> for DummyReceiver {
        fn set(self, value: T) {
            std::println!("received: {:?}", value);
        }
    }

    #[test]
    fn it_works() {
        let s = and_then(map(value(1), |i| i + 1), |t| value(t + 2));
        let op = pown!(s.connect(DummyReceiver));
        OperationState::start(op);
    }
}
