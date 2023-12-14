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
    type Execution = Exec<S, Sched, R>;

    fn connect(self, receiver: R) -> Self::Execution {
        Exec {
            sched: self.1,
            sender: self.0,
            receiver,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Exec<S, Sched, R> {
    sched: Sched,
    sender: S,
    receiver: R,
}

impl<S, Sched, R> Execution for Exec<S, Sched, R>
where
    S: SenderTo<R>,
    R: ReceiverFrom<S>,
    Sched: Scheduler,
    Sched::Sender: SenderTo<Recv<S, R>>,
{
    fn execute(self) {
        let receiver = Recv {
            sender: self.sender,
            receiver: self.receiver,
        };
        self.sched.schedule().connect(receiver).execute()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Recv<S, R> {
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
