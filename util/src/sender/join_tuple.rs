mod aux;

use either_slot::{
    array::Element,
    tuple::{self, Concat, Count, CountOf, InElement, Index, List, Place, TakeList, Whole},
};
use tuple_list::{Tuple, TupleList};

use self::aux::{ConstructReceiver, ReceiverList, ZipOption};
use rxec_core::{
    tuple_list::{ExecutionList, SenderList, SenderListTo},
    Execution, Receiver, Sender, SenderTo,
};

pub fn join_tuple<S>(s: S) -> JoinTuple<S>
where
    S: Tuple,
    S::TupleList: SenderList,
{
    JoinTuple(s)
}

#[macro_export]
macro_rules! join {
    ($($e:expr),* $(,)?) => {
        $crate::sender::join_tuple(($($e,)*))
    };
}

pub trait JoinTupleExt: Tuple {
    fn join(self) -> JoinTuple<Self>
    where
        Self::TupleList: SenderList,
    {
        join_tuple(self)
    }
}
impl<T: Tuple> JoinTupleExt for T {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JoinTuple<S>(S);

impl<S> Sender for JoinTuple<S>
where
    S: Tuple,
    S::TupleList: SenderList,
{
    type Output = <<S::TupleList as SenderList>::Output as TupleList>::Tuple;
}

impl<R, S, Z, O> SenderTo<R> for JoinTuple<S>
where
    R: Receiver<O>,
    S: Tuple,
    S::TupleList: SenderListTo<Z>,
    <S::TupleList as SenderList>::Output: TupleList<Tuple = O>,
    Z: TupleList,

    O: Concat<(R,)> + ConstructReceiver<R, Receiver = Z>,
    O::Output: Concat<()>,
    List<O, R, ()>: InElement,
    Whole<O, R, ()>: ReceiverList,
    TakeList<O, R, ()>: ZipOption<Zipped = List<O, R, ()>>,

    O::TupleList: Count,
    Place<O, R, ()>: Index<CountOf<O>, Output = Element<R>>,
{
    type Execution = Exec<R, S::TupleList, Z, O>;

    fn connect(self, receiver: R) -> Self::Execution {
        let (rr, rlist) = O::construct();
        let elist = self.0.into_tuple_list().connect_list(rlist);
        Exec {
            elist,
            receiver,
            rr,
        }
    }
}

#[derive(Debug)]
pub struct Exec<R, S, Z, O>
where
    R: Receiver<O>,
    S: SenderListTo<Z>,
    S::Output: TupleList<Tuple = O>,
    Z: TupleList,

    O: Concat<(R,)>,
    O::Output: Concat<()>,
    List<O, R, ()>: InElement,
{
    elist: S::ExecutionList,
    receiver: R,
    rr: tuple::Sender<O, R, ()>,
}

impl<R, S, Z, O> Execution for Exec<R, S, Z, O>
where
    R: Receiver<O>,
    S: SenderListTo<Z>,
    S::Output: TupleList<Tuple = O>,
    Z: TupleList,

    O: Concat<(R,)>,
    O::Output: Concat<()>,
    List<O, R, ()>: InElement,
    Whole<O, R, ()>: ReceiverList,
    TakeList<O, R, ()>: ZipOption<Zipped = List<O, R, ()>>,

    O::TupleList: Count,
    Place<O, R, ()>: Index<CountOf<O>, Output = Element<R>>,
{
    fn execute(self) {
        self.elist.execute_list();
        if let Err(rr) = self.rr.send(self.receiver) {
            if let Some(tlist) = rr.into_tuple_list().zip_option() {
                ReceiverList::consume(tlist.into_tuple());
            }
        }
    }
}

pub struct Recv<R, Head, Current, Tail>
where
    Head: Concat<(Current,)>,
    Tail: Concat<(R,)>,
    Head::Output: Concat<Tail::Output>,
    List<Head, Current, Tail::Output>: InElement,
{
    rr: tuple::Sender<Head, Current, <Tail as Concat<(R,)>>::Output>,
}

impl<R, Head, Current, Tail> Receiver<Current> for Recv<R, Head, Current, Tail>
where
    Head: Concat<(Current,)>,
    Tail: Concat<(R,)>,
    Head::Output: Concat<Tail::Output> + Concat<Tail>,
    List<Head, Current, Tail::Output>: InElement,
    TakeList<Head, Current, Tail::Output>: ZipOption<Zipped = List<Head, Current, Tail::Output>>,

    <Head as Tuple>::TupleList: Count,
    Place<Head, Current, Tail::Output>: Index<CountOf<Head>, Output = Element<Current>>,

    R: Receiver<Whole<Head, Current, Tail>>,
    Whole<Head, Current, Tail::Output>: ReceiverList,
{
    fn receive(self, value: Current) {
        if let Err(rr) = self.rr.send(value) {
            if let Some(tlist) = rr.into_tuple_list().zip_option() {
                ReceiverList::consume(tlist.into_tuple());
            }
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use std::string::ToString;

    use crate::{
        join,
        sender::{value, wait, SenderExt},
    };

    #[test]
    fn t() {
        let v1 = value(1);
        let v2 = value('c').map(|c| c.to_string());
        let v3 = value("123");

        let jt = join!(v1, v2, v3);
        assert_eq!(wait(jt), (1, 'c'.to_string(), "123"));
    }
}
