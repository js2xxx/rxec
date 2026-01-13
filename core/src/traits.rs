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
    /// - Although this object is `mem::forget`ed after this function is called.
    unsafe fn start_by_ref(self: Pin<&mut Self>);

    /// Start the operation.
    fn start(mut this: POwn<'_, Self>) {
        // SAFETY:
        //
        // - This function is called only once for this object, since it transfers the
        //   ownership via `POwn`.
        // - Although `this` is `mem::forget`ed, its destructor will run in the
        //   associated `DropSlot`, which will properly drop the operation and satisfy
        //   the pinning guarantee.
        unsafe {
            this.as_mut().start_by_ref();
            mem::forget(this);
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
