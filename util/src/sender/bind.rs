use rxec_core::{Execution, Receiver, ReceiverFrom, Sender, SenderTo};

pub fn bind<S, F>(s: S, f: F) -> Bind<S, F> {
    Bind(s, f)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bind<S, F>(S, F);

impl<S, F, T> Sender for Bind<S, F>
where
    S: Sender,
    F: FnOnce(S::Output) -> T,
    T: Sender,
{
    type Output = T::Output;
}

impl<S, F, T, R> SenderTo<R> for Bind<S, F>
where
    S: SenderTo<Recv<F, R>>,
    F: FnOnce(S::Output) -> T,
    T: SenderTo<R>,
    R: ReceiverFrom<T>,
{
    type Execution = S::Execution;

    fn connect(self, receiver: R) -> Self::Execution {
        self.0.connect(Recv { receiver, f: self.1 })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Recv<F, R> {
    receiver: R,
    f: F,
}

impl<F, R, O, T> Receiver<O> for Recv<F, R>
where
    F: FnOnce(O) -> T,
    T: SenderTo<R>,
    R: ReceiverFrom<T>,
{
    fn receive(self, value: O) {
        (self.f)(value).connect(self.receiver).execute()
    }
}
