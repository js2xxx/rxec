use alloc::{sync::Arc, task::Wake};
use core::{
    future::Future,
    hint,
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::Deref,
    pin::Pin,
    ptr,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
    task::{Context, Poll, Waker},
};

use spin::Mutex;

use crate::{Execution, Receiver, ReceiverFrom, SenderTo};

struct Inner<F, R> {
    fut: F,
    receiver: Option<R>,
}

struct CpsImpl<F, R> {
    inner: Mutex<Inner<F, R>>,
    starve: AtomicUsize,
}

const STARVE_MAX: usize = isize::MAX as usize;
const COMPLETE: usize = STARVE_MAX + (STARVE_MAX >> 1);

impl<F, R> CpsImpl<F, R>
where
    F: Future + Send + 'static,
    R: Receiver<F::Output> + Send + 'static,
{
    fn try_poll(self: &Arc<Self>, first: bool) -> bool {
        if let Some(mut inner) = self.inner.try_lock() {
            let waker = WakerRef::from(self);
            let fut = unsafe { Pin::new_unchecked(&mut inner.fut) };
            if let Poll::Ready(output) = fut.poll(&mut Context::from_waker(&waker)) {
                inner.receiver.take().unwrap().receive(output);
                self.starve.store(COMPLETE, Relaxed);
                return false;
            }
            let starve = self.starve.swap(0, Relaxed);
            return starve > (first as usize);
        }
        false
    }
}

pub struct Cps<F, R>(Arc<CpsImpl<F, R>>);

impl<F, R> Execution for Cps<F, R>
where
    F: Future + Send + 'static,
    R: Receiver<F::Output> + Send + 'static,
{
    fn execute(self) {
        self.0.wake()
    }
}

impl<F, R> Wake for CpsImpl<F, R>
where
    F: Future + Send + 'static,
    R: Receiver<F::Output> + Send + 'static,
{
    fn wake(self: Arc<Self>) {
        if self.starve.fetch_add(1, Relaxed) >= COMPLETE {
            self.starve.store(COMPLETE, Relaxed);
            return;
        }
        if self.try_poll(true) {
            while self.starve.load(Relaxed) < COMPLETE && self.try_poll(false) {
                hint::spin_loop()
            }
        }
    }
}

impl<F: Future + Send + 'static, R> SenderTo<R> for F
where
    R: ReceiverFrom<F> + Send + 'static,
{
    type Execution = Cps<F, R>;

    fn connect(self, receiver: R) -> Self::Execution {
        Cps(Arc::new(CpsImpl {
            inner: Mutex::new(Inner {
                fut: self,
                receiver: Some(receiver),
            }),
            starve: AtomicUsize::new(0),
        }))
    }
}

pub trait CpsExt: Future + Send + Sized + 'static {
    fn cps<R>(self, continuation: R) -> Cps<Self, R>
    where
        R: ReceiverFrom<Self> + Send + 'static,
    {
        self.connect(continuation)
    }
}

struct WakerRef<'a> {
    inner: ManuallyDrop<Waker>,
    marker: PhantomData<&'a Waker>,
}

impl<'a, W: Wake + Send + Sync + 'static> From<&'a Arc<W>> for WakerRef<'a> {
    fn from(value: &'a Arc<W>) -> Self {
        unsafe {
            let copy = ptr::read(value);
            WakerRef {
                inner: ManuallyDrop::new(Waker::from(copy)),
                marker: PhantomData,
            }
        }
    }
}

impl Deref for WakerRef<'_> {
    type Target = Waker;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
