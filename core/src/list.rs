use core::pin::Pin;

use pin_project::pin_project;
use placid::prelude::*;
use tsum::sum::{
    index::{UInt, UTerm},
    repr::SumList,
};

use crate::traits::Sender;

pub trait UIndex = tsum::sum::index::Index;

pub trait USub<U: UIndex>: UIndex {
    type Output: UIndex;
}
pub type USubT<U, V> = <U as USub<V>>::Output;

impl<U: UIndex> USub<UTerm> for U {
    type Output = U;
}
impl<U: USub<V>, V: UIndex> USub<UInt<V>> for UInt<U> {
    type Output = USubT<U, V>;
}

pub trait CountList {
    type Count: UIndex;
}
pub type CountListT<L> = <L as CountList>::Count;

impl CountList for () {
    type Count = UTerm;
}

impl<Head, Tail: CountList> CountList for (Head, Tail) {
    type Count = UInt<Tail::Count>;
}

pub trait IndexList<U: UIndex>: CountList {
    type Output;
}
pub type IndexListT<L, U> = <L as IndexList<U>>::Output;

impl<Head, Tail: CountList> IndexList<UTerm> for (Head, Tail) {
    type Output = Head;
}

impl<Head, Tail: IndexList<U>, U: UIndex> IndexList<UInt<U>> for (Head, Tail) {
    type Output = Tail::Output;
}

pub trait PinnedList {
    type TupleList: CountList;

    fn from_tuple_list(list: Self::TupleList) -> Self;
}

#[derive(InitPin)]
pub struct PTerm;

impl PinnedList for PTerm {
    type TupleList = ();

    fn from_tuple_list(_list: ()) -> Self {
        PTerm
    }
}

#[derive(InitPin)]
#[pin_project(project = PConsProj)]
pub struct PCons<Head, Tail>(#[pin] pub Head, #[pin] pub Tail);

impl<Head, Tail: PinnedList> PinnedList for PCons<Head, Tail> {
    type TupleList = (Head, Tail::TupleList);

    fn from_tuple_list(list: Self::TupleList) -> Self {
        PCons(list.0, Tail::from_tuple_list(list.1))
    }
}

pub trait SenderList: CountList {
    type OutputList: SumList;
}
pub type SenderOutputList<L> = <L as SenderList>::OutputList;

impl SenderList for () {
    type OutputList = ();
}

impl<Head, Tail> SenderList for (Head, Tail)
where
    Head: Sender,
    Tail: SenderList,
{
    type OutputList = (Head::Output, Tail::OutputList);
}

pub trait OperationStateList: PinnedList {
    fn start_all(self: Pin<&mut Self>);
}

impl OperationStateList for PTerm {
    fn start_all(self: Pin<&mut Self>) {}
}

impl<Head, Tail> OperationStateList for PCons<Head, Tail>
where
    Head: crate::traits::OperationState,
    Tail: OperationStateList,
{
    fn start_all(self: Pin<&mut Self>) {
        let PConsProj(head, tail) = self.project();
        head.start();
        tail.start_all();
    }
}
