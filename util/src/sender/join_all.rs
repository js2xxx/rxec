use alloc::vec::Vec;
use core::iter::TrustedLen;

use either_slot::array;

use rxec_core::{Execution, Receiver, Sender, SenderTo};

pub fn join_all<S, I>(iter: I) -> JoinAll<S, I>
where
    S: Sender,
    I: IntoIterator<Item = S>,
{
    JoinAll(iter)
}

pub trait JoinAllExt: IntoIterator + Sized {
    fn join_all(self) -> JoinAll<Self::Item, Self>
    where
        Self::Item: Sender,
    {
        join_all(self)
    }
}
impl<I: IntoIterator + Sized> JoinAllExt for I {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JoinAll<S, I>(I)
where
    S: Sender,
    I: IntoIterator<Item = S>;

impl<S, I> Sender for JoinAll<S, I>
where
    S: Sender,
    I: IntoIterator<Item = S>,
{
    type Output = Vec<S::Output>;
}

impl<S, I, R> SenderTo<R> for JoinAll<S, I>
where
    S: SenderTo<Recv<S, R>>,
    I: IntoIterator<Item = S>,
    R: Receiver<Vec<S::Output>>,
{
    type Execution = Exec<S, R>;

    fn connect(self, receiver: R) -> Self::Execution {
        SpecConnect::spec_connect(self.0.into_iter(), receiver)
    }
}

type El<S, R> = array::Sender<
    Data<<S as Sender>::Output, R>,
    Vec<array::Element<Data<<S as Sender>::Output, R>>>,
>;

pub struct Exec<S, R>
where
    S: SenderTo<Recv<S, R>>,
    R: Receiver<Vec<S::Output>>,
{
    exec: Vec<S::Execution>,
    sender: El<S, R>,
    receiver: R,
}

pub(super) trait SpecConnect<S, I, R> {
    fn spec_connect(sender: I, receiver: R) -> Self;
}

impl<S, I, R> SpecConnect<S, I, R> for Exec<S, R>
where
    S: SenderTo<Recv<S, R>>,
    R: Receiver<Vec<S::Output>>,
    I: Iterator<Item = S>,
{
    default fn spec_connect(sender: I, receiver: R) -> Self {
        let sender = sender.collect::<Vec<_>>();
        SpecConnect::spec_connect(sender.into_iter(), receiver)
    }
}

impl<S, I, R> SpecConnect<S, I, R> for Exec<S, R>
where
    S: SenderTo<Recv<S, R>>,
    R: Receiver<Vec<S::Output>>,
    I: TrustedLen<Item = S>,
{
    fn spec_connect(sender: I, receiver: R) -> Self {
        let mut el = array::vec(sender.size_hint().0 + 1);
        Exec {
            exec: sender
                .zip(el.by_ref())
                .map(|(s, el)| s.connect(Recv { sender: el }))
                .collect(),
            sender: el.next().unwrap(),
            receiver,
        }
    }
}

fn cont<T, R>(data: impl Iterator<Item = Data<T, R>>)
where
    R: Receiver<Vec<T>>,
{
    let cap = data.size_hint().1.unwrap_or(0);
    let mut elems = Vec::with_capacity(cap.saturating_sub(1));
    let mut receiver = None;
    data.for_each(|elem| match elem {
        Data::Element(data) => elems.push(data),
        Data::Receiver(r) => receiver = Some(r),
    });
    if let Some(receiver) = receiver {
        receiver.receive(elems)
    }
}

impl<S, R> Execution for Exec<S, R>
where
    S: SenderTo<Recv<S, R>>,
    R: Receiver<Vec<S::Output>>,
{
    fn execute(self) {
        self.exec.execute();
        let value = Data::Receiver(self.receiver);
        if let Err(data) = self.sender.send(value) {
            cont(data)
        }
    }
}

enum Data<T, R> {
    Element(T),
    Receiver(R),
}

pub struct Recv<S, R>
where
    S: SenderTo<Recv<S, R>>,
    R: Receiver<Vec<S::Output>>,
{
    sender: El<S, R>,
}

impl<S, R> Receiver<S::Output> for Recv<S, R>
where
    S: SenderTo<Recv<S, R>>,
    R: Receiver<Vec<S::Output>>,
{
    fn receive(self, value: S::Output) {
        let value = Data::Element(value);
        if let Err(data) = self.sender.send(value) {
            cont(data)
        }
    }
}
