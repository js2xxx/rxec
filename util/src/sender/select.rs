use alloc::sync::Arc;
use core::marker::PhantomData;

use either::Either::{self, Left, Right};
use rxec_core::{Receiver, ReceiverFrom, Sender, SenderTo};
use spin::Mutex;

pub fn select<S1, S2>(s1: S1, s2: S2) -> Select<S1, S2>
where
    S1: Sender,
    S2: Sender,
{
    Select(s1, s2)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Select<S1, S2>(S1, S2);

impl<S1, S2> Sender for Select<S1, S2>
where
    S1: Sender,
    S2: Sender,
{
    type Output = Either<S1::Output, S2::Output>;
}

impl<S1, S2, R> SenderTo<R> for Select<S1, S2>
where
    S1: SenderTo<Recv<S1, S2, R, false>>,
    S2: SenderTo<Recv<S1, S2, R, true>>,
    R: ReceiverFrom<Select<S1, S2>>,
{
    type Execution = (S1::Execution, S2::Execution);

    fn connect(self, receiver: R) -> Self::Execution {
        let shared = Arc::new(Mutex::new(Some(receiver)));
        let e1 = self.0.connect(Recv {
            shared: shared.clone(),
            marker: PhantomData,
        });
        let e2 = self.1.connect(Recv {
            shared,
            marker: PhantomData,
        });
        (e1, e2)
    }
}

#[derive(Debug, Clone)]
pub struct Recv<S1, S2, R, const EITHER: bool>
where
    S1: SenderTo<Recv<S1, S2, R, false>>,
    S2: SenderTo<Recv<S1, S2, R, true>>,
    R: ReceiverFrom<Select<S1, S2>>,
{
    shared: Arc<Mutex<Option<R>>>,
    marker: PhantomData<fn(S1::Output, S2::Output)>,
}

impl<S1, S2, R> Receiver<S1::Output> for Recv<S1, S2, R, false>
where
    S1: SenderTo<Recv<S1, S2, R, false>>,
    S2: SenderTo<Recv<S1, S2, R, true>>,
    R: ReceiverFrom<Select<S1, S2>>,
{
    fn receive(self, value: S1::Output) {
        let Some(receiver) = self.shared.lock().take() else {
            return;
        };
        receiver.receive(Left(value));
    }
}

impl<S1, S2, R> Receiver<S2::Output> for Recv<S1, S2, R, true>
where
    S1: SenderTo<Recv<S1, S2, R, false>>,
    S2: SenderTo<Recv<S1, S2, R, true>>,
    R: ReceiverFrom<Select<S1, S2>>,
{
    fn receive(self, value: S2::Output) {
        let Some(receiver) = self.shared.lock().take() else {
            return;
        };
        receiver.receive(Right(value));
    }
}
