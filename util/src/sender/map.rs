use rxec_core::{Receiver, Sender, SenderTo};

pub fn map<S, F>(s: S, f: F) -> Map<S, F> {
    Map(s, f)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Map<S, F>(S, F);

impl<S, F, U> Sender for Map<S, F>
where
    S: Sender,
    F: FnOnce(S::Output) -> U,
{
    type Output = F::Output;
}

impl<S, F, U, R> SenderTo<R> for Map<S, F>
where
    S: SenderTo<Recv<F, R>>,
    F: FnOnce(S::Output) -> U,
    R: Receiver<U>,
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

impl<F, R, T, U> Receiver<T> for Recv<F, R>
where
    F: FnOnce(T) -> U,
    R: Receiver<U>,
{
    fn receive(self, value: T) {
        self.receiver.receive((self.f)(value))
    }
}
