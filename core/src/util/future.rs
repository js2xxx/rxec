use alloc::sync::Arc;
use core::{
    convert::Infallible,
    marker::PhantomData,
    mem::ManuallyDrop,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering::*},
    task::*,
};

use placid::prelude::*;
use spin::Mutex;

use crate::{Receiver, basic::*};

pub struct FutureExpr<F>(PhantomData<F>);

struct FutureData<F, R>
where
    F: Future + Send,
    R: Receiver<F::Output> + Send,
{
    f: F,
    recv: Option<R>,
}

struct FutureStateInner<F, R>
where
    F: Future + Send,
    R: Receiver<F::Output> + Send,
{
    inner: Mutex<Option<FutureData<F, R>>>,
    polls_again: AtomicBool,
}

/// ```rust,compile_fail
/// fn assert_send<T: Send>() {}
///
/// fn for_future<F: Future>() {
///     assert_send::<rxec_core::util::Async<F>>();
/// }
/// ```
///
/// ```rust
/// fn assert_send<T: Send>() {}
///
/// fn for_future<F: Future + Send>() {
///     assert_send::<rxec_core::util::Async<F>>();
/// }
/// ```
struct _TestSendInheritance;

impl<F, R, T> FutureStateInner<F, R>
where
    F: Future<Output = T> + Send,
    R: Receiver<T> + Send,
{
    // SAFETY on Wakers:
    //
    // - For `Waker: Send + Sync` which requires `F, R: Send`, we ensure this
    //   naturally;
    // - For `Waker: 'static` which requires `F, R: 'static`, we don't actually
    //   require this because a clone of `Self` is contained by the subsequent
    //   operation state graph, which is guaranteed to drop after start before the
    //   future is polled. Thus, the future and receiver are properly dropped by
    //   `Drop` impl of `MainFutureState`, so they don't escape their lifetimes.
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        // fn clone(self: &Arc<Self>) -> Arc<Self>
        |data| {
            // SAFETY: Arc::clone(&self) is equivalent to incrementing the strong count.
            unsafe { Arc::increment_strong_count(data.cast::<Self>()) };
            RawWaker::new(data, &Self::VTABLE)
        },
        // fn wake(self: Arc<Self>)
        |data| {
            // SAFETY: Keeping the signature `fn wake(self: Arc<Self>)`
            let arc = unsafe { Arc::from_raw(data.cast::<Self>()) };
            if !arc.try_poll() {
                arc.polls_again.store(true, Release);
            }
        },
        |data| {
            // SAFETY: Keeping the signature `fn wake_by_ref(self: &Arc<Self>)`
            let arc = unsafe { ManuallyDrop::new(Arc::from_raw(data.cast::<Self>())) };
            if !arc.try_poll() {
                arc.polls_again.store(true, Release);
            }
        },
        // fn drop(self: Arc<Self>)
        // SAFETY: Arc::drop(self) decrements the strong count.
        |data| unsafe { Arc::decrement_strong_count(data.cast::<Self>()) },
    );

    fn try_poll(self: &Arc<Self>) -> bool {
        let Some(mut inner_opt) = self.inner.try_lock() else {
            return false;
        };
        let Some(inner) = &mut *inner_opt else {
            return false;
        };

        while self.polls_again.swap(false, Acquire) {
            // SAFETY: Arc<Self> is a valid waker.
            let waker =
                unsafe { ManuallyDrop::new(Waker::new(Arc::as_ptr(self).cast(), &Self::VTABLE)) };
            let mut cx = Context::from_waker(&waker);

            // SAFETY: We don't move out `f`.
            match unsafe { Pin::new_unchecked(&mut inner.f) }.poll(&mut cx) {
                Poll::Ready(output) => {
                    let recv = inner.recv.take().expect("future polled after completion");
                    recv.set(output);
                    *inner_opt = None;
                    return true;
                }
                Poll::Pending => {}
            }
        }

        true
    }
}

impl<F, T> SenderExpr for FutureExpr<F>
where
    F: Future<Output = T> + Send,
{
    type Output = T;
    type Data = F;
    type SubSenders = ();
}

pub struct FutureState<F, R>(Arc<FutureStateInner<F, R>>)
where
    F: Future + Send,
    R: Receiver<F::Output> + Send;

impl<F, R> Drop for FutureState<F, R>
where
    F: Future + Send,
    R: Receiver<F::Output> + Send,
{
    fn drop(&mut self) {
        // Drop the future and receiver to cancel the operation, and make sure
        // they don't escape their lifetimes.
        *self.0.inner.lock() = None;
    }
}

impl<F, R, T> SenderExprTo<R> for FutureExpr<F>
where
    F: Future<Output = T> + Send,
    R: Receiver<T> + Send,
{
    type State = FutureState<F, R>;
    type Error = Infallible;
    type CreateState = impl InitPin<Self::State, Error = Self::Error>;

    fn create_state(f: Self::Data, _: &mut (), recv: R) -> Self::CreateState {
        init::with(|| {
            FutureState(Arc::new(FutureStateInner {
                inner: Mutex::new(Some(FutureData { f, recv: Some(recv) })),
                polls_again: AtomicBool::new(true),
            }))
        })
    }

    fn start(state: StateRef<'_, Self, R>, _: Pin<&mut ConnectAllOps<Self, R>>)
    where
        State<Self, R>: ConnectAll<Self, R>,
    {
        state.state_mut().0.try_poll();
    }

    fn complete(_: StateRef<'_, Self, R>, value: tsum::Sum<()>) {
        value.unreachable();
    }
}

pub type Async<F> = BasicSender<FutureExpr<F>>;

pub fn async_<F, T>(fut: F) -> Async<F>
where
    F: Future<Output = T> + Send,
{
    BasicSender::new(fut, ())
}

#[cfg(test)]
mod tests {
    use core::{cell::Cell, task::Waker};

    use placid::pown;

    use crate::{OperationState, Receiver, SenderTo, util::*};

    struct DummyReceiver;

    impl<T: core::fmt::Debug> Receiver<T> for DummyReceiver {
        fn set(self, value: T) {
            std::println!("received: {:?}", value);
        }
    }

    std::thread_local! {
        static WAKER: Cell<Option<Waker>> = const { Cell::new(None) };
        static DROPPED: Cell<bool> = const { Cell::new(false) };
    }

    struct TestFuture(Option<i32>);

    impl Future for TestFuture {
        type Output = i32;
        fn poll(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<Self::Output> {
            WAKER.set(Some(cx.waker().clone()));
            match self.get_mut().0.take() {
                Some(v) => core::task::Poll::Ready(v),
                None => core::task::Poll::Pending,
            }
        }
    }

    impl Drop for TestFuture {
        fn drop(&mut self) {
            DROPPED.set(true);
        }
    }

    #[test]
    fn it_works() {
        {
            // A immediately ready future
            let s = and_then(map(async_(TestFuture(Some(1))), |i| i + 1), |t| {
                value(t + 2)
            });
            let op = pown!(s.connect(DummyReceiver));
            OperationState::start(op);
        }
        assert!(DROPPED.replace(false));
        let waker = WAKER.replace(None).unwrap();
        waker.wake();

        {
            // Mimicking a future that is slow enough for us to
            // cancel it before it completes (when would be out
            // of our lifetime).
            let s = and_then(map(async_(TestFuture(None)), |i| i + 1), |t| value(t + 2));
            let op = pown!(s.connect(DummyReceiver));
            OperationState::start(op);
        }
        assert!(DROPPED.replace(false));
        let waker = WAKER.replace(None).unwrap();
        waker.wake();
    }
}
