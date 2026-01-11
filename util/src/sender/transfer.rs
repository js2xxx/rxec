use rxec_core::{Execution, Receiver, Sender, SenderTo};

use super::Scheduler;

pub fn transfer<S, Sched>(s: S, sched: Sched) -> Transfer<S, Sched> {
    Transfer(s, sched)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Transfer<S, Sched>(S, Sched);

impl<S, Sched> Sender for Transfer<S, Sched>
where
    S: Sender,
    Sched: Scheduler,
{
    type Output = S::Output;
}

impl<S, Sched, R> SenderTo<R> for Transfer<S, Sched>
where
    S: SenderTo<Local<R, Sched>>,
    R: Receiver<S::Output>,
    Sched: Scheduler,
    Sched::Sender: SenderTo<Remote<R, S::Output>>,
{
    type Execution = S::Execution;

    fn connect(self, receiver: R) -> Self::Execution {
        self.0.connect(Local { receiver, sched: self.1 })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Local<R, Sched> {
    receiver: R,
    sched: Sched,
}

impl<R, Sched, T> Receiver<T> for Local<R, Sched>
where
    R: Receiver<T>,
    Sched: Scheduler,
    Sched::Sender: SenderTo<Remote<R, T>>,
{
    fn receive(self, value: T) {
        let remote = Remote { receiver: self.receiver, value };
        self.sched.schedule().connect(remote).execute()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Remote<R, T> {
    receiver: R,
    value: T,
}

impl<R, T> Receiver<()> for Remote<R, T>
where
    R: Receiver<T>,
{
    fn receive(self, _: ()) {
        self.receiver.receive(self.value)
    }
}
