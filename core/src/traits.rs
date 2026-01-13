use core::{mem, pin::Pin};

use placid::{init::InitPin, pin::POwn};

pub trait Scheduler {
    type Task: Sender<Output = ()>;

    fn schedule(&self) -> Self::Task;
}

pub trait Receiver<T> {
    fn set(self, value: T);
}

pub trait ReceiverFrom<S: Sender + ?Sized>: Receiver<S::Output> {}
impl<T, S: Sender> ReceiverFrom<S> for T where T: Receiver<S::Output> {}

pub trait OperationState {
    /// Start the operation by reference.
    ///
    /// # Safety
    ///
    /// - This function must be called only once for this object.
    /// - This object must not be `mem::forget`ed after this function is called.
    unsafe fn start_by_ref(self: Pin<&mut Self>);

    /// Start the operation.
    ///
    /// Although this function takes ownership of the operation state, it would
    /// remain available after this function returns, and would be properly
    /// dropped in the current scope.
    ///
    /// Note that `op` is better not a temporary value constructed by `pown!` or
    /// `into_pown!`, since the object would be dropped immediately after this
    /// function returns, canceling the operation.
    ///
    /// # Examples
    ///
    /// Recommended way to start an operation:
    ///
    /// ```ignore
    /// struct WaitReceiver(Thread);
    ///
    /// impl<T> Receiver<T> for WaitReceiver {
    ///     fn set(self, _: T) {
    ///         received = true;
    ///         self.0.unpark();
    ///     }
    /// }
    ///
    /// fn wait<S: SenderTo<WaitReceiver>>(sender: S) {
    ///     let init = sender.connect(WaitReceiver(thread::current()));
    ///     {
    ///         let op = pown!(init);
    ///         OperationState::start(op);
    ///         // The operation is now running.
    ///         // Wait until the value is received.
    ///         while !received { thread::park(); }
    ///         // Drop the operation here after finished.
    ///     }
    ///     // Get the value here.
    /// }
    /// ```
    fn start(mut op: POwn<'_, Self>) {
        // SAFETY:
        //
        // - This function is called only once for this object, since it transfers the
        //   ownership via `POwn`.
        // - Although `op` is `mem::forget`ed, its destructor will run in the associated
        //   `DropSlot`, which will properly drop the operation and satisfy the pinning
        //   guarantee.
        unsafe {
            op.as_mut().start_by_ref();
            mem::forget(op);
        }
    }
}

pub trait Sender {
    type Output;
}
pub type SenderOutput<S> = <S as Sender>::Output;

pub trait SenderTo<Recv: Receiver<Self::Output>>: Sender {
    type Operation: OperationState;
    type ConnectError: core::fmt::Debug;

    fn connect(self, receiver: Recv) -> impl InitPin<Self::Operation, Error = Self::ConnectError>;
}
pub type ConnectOp<S, R> = <S as SenderTo<R>>::Operation;
