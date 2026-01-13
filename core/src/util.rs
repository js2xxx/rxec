mod and_then;
mod map;
mod value;

pub use self::{
    and_then::{AndThen, and_then},
    map::{Map, map},
    value::{Value, value},
};

#[cfg(test)]
#[allow(unused)]
mod tests {
    use std::marker::PhantomData;

    use placid::pown;
    use tsum::{
        T,
        sum::index::{UInt, UTerm},
    };

    use super::*;
    use crate::{
        OperationState, Receiver, SenderTo,
        basic::{BasicReceiver, BasicSender, State, ConnectAll, SenderExpr, SenderExprTo},
        list::{CountList, CountListT},
        util::{map::MapExpr, value::ValueExpr},
    };

    struct DummyReceiver;

    impl<T: core::fmt::Debug> Receiver<T> for DummyReceiver {
        fn set(self, value: T) {
            std::println!("received: {:?}", value);
        }
    }

    #[test]
    fn it_works() {
        let s = and_then(map(value(1), |i| i + 1), |t| value(t + 2));
        let i = SenderTo::<DummyReceiver>::connect(s, DummyReceiver);
        let mut op = pown!(i);
        op.as_mut().start();
    }
}
