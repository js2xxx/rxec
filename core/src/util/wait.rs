use placid::prelude::*;

use crate::{OperationState, Receiver, SenderTo};

#[derive(Debug, thiserror::Error)]
#[error("the sender operation was cancelled")]
pub struct CanceledError;

pub struct WaitRecv<T>(oneshot::Sender<T>);

impl<T> Receiver<T> for WaitRecv<T> {
    fn set(self, value: T) {
        let _ = self.0.send(value);
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

    let op = pown!(sender.connect(WaitRecv(s)));
    OperationState::start(op);

    r.await.map_err(|_| CanceledError)
}

#[cfg(feature = "std")]
pub fn sync_wait<T, S>(sender: S) -> Result<T, CanceledError>
where
    S: SenderTo<WaitRecv<T>, Output = T>,
{
    let (s, r) = oneshot::channel();

    let op = pown!(sender.connect(WaitRecv(s)));
    OperationState::start(op);

    r.recv().map_err(|_| CanceledError)
}
