use rxec_core::{Execution, ReceiverFrom, Sender, SenderTo};

pub fn eager<S: Sender>(s: S) -> Eager<S> {
    Eager(s)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Eager<S: Sender>(S);

impl<S: Sender> Sender for Eager<S> {
    type Output = S::Output;
}

impl<S, R> SenderTo<R> for Eager<S>
where
    S: SenderTo<R>,
    R: ReceiverFrom<S>,
{
    type Execution = ();

    fn connect(self, receiver: R) {
        self.0.connect(receiver).execute();
    }
}
