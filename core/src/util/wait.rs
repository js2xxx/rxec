use core::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project::pin_project;
use placid::prelude::*;

use crate::{OperationState, Receiver, SenderTo, traits::ConnectOp};

#[derive(Debug, thiserror::Error)]
#[error("the sender operation was cancelled")]
pub struct CanceledError;

pub struct WaitRecv<T>(oneshot::Sender<T>);

impl<T> Receiver<T> for WaitRecv<T> {
    fn set(self, value: T) {
        let _ = self.0.send(value);
    }
}

#[derive(InitPin)]
#[pin_project]
pub struct Wait<T, S: SenderTo<WaitRecv<T>, Output = T>> {
    started: bool,
    #[pin]
    op: ConnectOp<S, WaitRecv<T>>,
    #[pin]
    recv: oneshot::Receiver<T>,
}

impl<T, S: SenderTo<WaitRecv<T>, Output = T>> Future for Wait<T, S> {
    type Output = Result<S::Output, CanceledError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if !*this.started {
            *this.started = true;
            // SAFETY: The operation is started only once here, and its drop guarantee is
            // satisfied by constructor.
            unsafe { this.op.start_by_ref() };
        }
        this.recv.poll(cx).map_err(|_| CanceledError)
    }
}

/// Waits for the sender to complete and returns the output value.
///
/// # Safety
///
/// The caller must ensure that the caller future is properly dropped, i.e., not
/// `mem::forget`ed.
pub async unsafe fn wait<T, S>(sender: S) -> Result<T, CanceledError>
where
    S: SenderTo<WaitRecv<T>, Output = T>,
{
    let (s, r) = oneshot::channel();
    let wait_recv = WaitRecv(s);

    let wait_fut: POwn<'_, Wait<T, S>> = pown!(init_pin!(Wait {
        started: init::value(false).adapt_err(),
        op: sender.connect(wait_recv),
        recv: init::value(r).adapt_err(),
    }));
    wait_fut.await
}

#[cfg(feature = "std")]
pub fn sync_wait<T, S>(sender: S) -> Result<T, CanceledError>
where
    S: SenderTo<WaitRecv<T>, Output = T>,
{
    let (s, r) = oneshot::channel();
    let wait_recv = WaitRecv(s);

    let op = pown!(sender.connect(wait_recv));
    OperationState::start(op);

    r.recv().map_err(|_| CanceledError)
}
