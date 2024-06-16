use either_slot::tuple;
use rxec_core::{Execution, Receiver, Sender, SenderTo};

pub fn join<S1, S2>(s1: S1, s2: S2) -> Join<S1, S2>
where
    S1: Sender,
    S2: Sender,
{
    Join(s1, s2)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Join<S1, S2>(S1, S2);

impl<S1, S2> Sender for Join<S1, S2>
where
    S1: Sender,
    S2: Sender,
{
    type Output = (S1::Output, S2::Output);
}

impl<S1, S2, R> SenderTo<R> for Join<S1, S2>
where
    S1: SenderTo<Recv1<S1, S2, R>>,
    S2: SenderTo<Recv2<S1, S2, R>>,
    R: Receiver<(S1::Output, S2::Output)>,
{
    type Execution = Exec<S1, S2, R>;

    fn connect(self, receiver: R) -> Self::Execution {
        let (r1, r2, rr) = tuple::<(S1::Output, S2::Output, R)>();
        let e1 = self.0.connect(Recv1 { r1 });
        let e2 = self.1.connect(Recv2 { r2 });
        Exec {
            executions: (e1, e2),
            receiver,
            rr,
        }
    }
}

#[derive(Debug)]
pub struct Exec<S1, S2, R>
where
    S1: SenderTo<Recv1<S1, S2, R>>,
    S2: SenderTo<Recv2<S1, S2, R>>,
    R: Receiver<(S1::Output, S2::Output)>,
{
    executions: (S1::Execution, S2::Execution),
    receiver: R,
    rr: tuple::Sender<(S1::Output, S2::Output), R, ()>,
}

impl<S1, S2, R> Execution for Exec<S1, S2, R>
where
    S1: SenderTo<Recv1<S1, S2, R>>,
    S2: SenderTo<Recv2<S1, S2, R>>,
    R: Receiver<(S1::Output, S2::Output)>,
{
    fn execute(self) {
        let (e1, e2) = self.executions;
        e1.execute();
        e2.execute();
        if let Err((Some(v1), Some(v2), Some(recv))) = self.rr.send(self.receiver) {
            recv.receive((v1, v2));
        }
    }
}

#[derive(Debug)]
pub struct Recv1<S1, S2, R>
where
    S1: SenderTo<Recv1<S1, S2, R>>,
    S2: SenderTo<Recv2<S1, S2, R>>,
    R: Receiver<(S1::Output, S2::Output)>,
{
    r1: tuple::Sender<(), S1::Output, (S2::Output, R)>,
}

impl<S1, S2, R> Receiver<S1::Output> for Recv1<S1, S2, R>
where
    S1: SenderTo<Recv1<S1, S2, R>>,
    S2: SenderTo<Recv2<S1, S2, R>>,
    R: Receiver<(S1::Output, S2::Output)>,
{
    fn receive(self, value: S1::Output) {
        if let Err((Some(v1), Some(v2), Some(recv))) = self.r1.send(value) {
            recv.receive((v1, v2));
        }
    }
}

#[derive(Debug)]
pub struct Recv2<S1, S2, R>
where
    S1: SenderTo<Recv1<S1, S2, R>>,
    S2: SenderTo<Recv2<S1, S2, R>>,
    R: Receiver<(S1::Output, S2::Output)>,
{
    r2: tuple::Sender<(S1::Output,), S2::Output, (R,)>,
}

impl<S1, S2, R> Receiver<S2::Output> for Recv2<S1, S2, R>
where
    S1: SenderTo<Recv1<S1, S2, R>>,
    S2: SenderTo<Recv2<S1, S2, R>>,
    R: Receiver<(S1::Output, S2::Output)>,
{
    fn receive(self, value: S2::Output) {
        if let Err((Some(v1), Some(v2), Some(recv))) = self.r2.send(value) {
            recv.receive((v1, v2));
        }
    }
}
