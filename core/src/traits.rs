use core::pin::Pin;

use placid::init::InitPin;

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
    fn start(self: Pin<&mut Self>);
}

pub trait Sender {
    type Output;
}
pub type SenderOutput<S> = <S as Sender>::Output;

pub trait SenderTo<Recv: Receiver<Self::Output>>: Sender {
    type Operation: OperationState;
    type ConnectError;

    fn connect(self, receiver: Recv) -> impl InitPin<Self::Operation, Error = Self::ConnectError>;
}
pub type ConnectOp<S, R> = <S as SenderTo<R>>::Operation;
