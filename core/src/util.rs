mod and_then;
mod future;
mod map;
mod value;

pub use self::{
    and_then::{AndThen, and_then},
    future::{Async, async_},
    map::{Map, map},
    value::{Value, value},
};

const ONESHOT_COMPLETED: &str = "oneshot sender already completed";

#[cfg(test)]
mod tests {
    use core::{cell::Cell, task::Waker};

    use placid::pown;

    use super::*;
    use crate::{OperationState, Receiver, SenderTo};

    struct DummyReceiver;

    impl<T: core::fmt::Debug> Receiver<T> for DummyReceiver {
        fn set(self, value: T) {
            std::println!("received: {:?}", value);
        }
    }

    std::thread_local! {
        static WAKER: Cell<Option<Waker>> = const { Cell::new(None) };
    }

    struct TestFuture;

    impl Future for TestFuture {
        type Output = i32;
        fn poll(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<Self::Output> {
            WAKER.with(|w| w.set(Some(cx.waker().clone())));
            core::task::Poll::Ready(42)
        }
    }

    #[test]
    fn it_works() {
        {
            let s = and_then(map(async_(TestFuture), |i| i + 1), |t| value(t + 2));
            let op = pown!(s.connect(DummyReceiver));
            OperationState::start(op);
        }
        let waker = WAKER.replace(None).unwrap();
        waker.wake();
    }
}
