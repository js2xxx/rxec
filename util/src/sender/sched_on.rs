use rxec_core::{Execution, Receiver, ReceiverFrom, Sender, SenderTo};

use super::Scheduler;

pub fn sched_on<S, Sched>(s: S, sched: Sched) -> SchedOn<S, Sched> {
    SchedOn(s, sched)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchedOn<S, Sched>(S, Sched);

impl<S, Sched> Sender for SchedOn<S, Sched>
where
    S: Sender,
    Sched: Scheduler,
{
    type Output = S::Output;
}

impl<S, Sched, R> SenderTo<R> for SchedOn<S, Sched>
where
    S: SenderTo<R>,
    R: ReceiverFrom<S>,
    Sched: Scheduler,
    Sched::Sender: SenderTo<Recv<S, R>>,
{
    type Execution = <Sched::Sender as SenderTo<Recv<S, R>>>::Execution;

    fn connect(self, receiver: R) -> Self::Execution {
        self.1.schedule().connect(Recv {
            sender: self.0,
            receiver,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Recv<S, R> {
    sender: S,
    receiver: R,
}

impl<S, R> Receiver<()> for Recv<S, R>
where
    S: SenderTo<R>,
    R: ReceiverFrom<S>,
{
    fn receive(self, _: ()) {
        self.sender.connect(self.receiver).execute();
    }
}
