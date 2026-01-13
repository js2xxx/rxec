mod and_then;
mod map;
mod value;

pub use self::{
    and_then::{AndThen, and_then},
    map::{Map, map},
    value::{Value, value},
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
        let i = s.connect(DummyReceiver);
        let mut op = pown!(i);
        op.as_mut().start();
    }
}
