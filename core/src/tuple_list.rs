use tuple_list::TupleList;

use crate::{Execution, ReceiverFrom, Sender, SenderTo};

pub trait SenderList: TupleList {
    type Output: TupleList;
}

impl SenderList for () {
    type Output = ();
}

impl<Head, Tail> SenderList for (Head, Tail)
where
    Tail: SenderList,
    Head: Sender,
    (Head, Tail): TupleList,
    (Head::Output, Tail::Output): TupleList,
{
    type Output = (Head::Output, Tail::Output);
}

pub trait SenderListTo<R: TupleList>: SenderList {
    type ExecutionList: ExecutionList;

    fn connect_list(self, receiver: R) -> Self::ExecutionList;
}

impl SenderListTo<()> for () {
    type ExecutionList = ();

    fn connect_list(self, _: ()) {}
}

impl<HeadSender, TailSender, HeadReceiver, TailReceiver> SenderListTo<(HeadReceiver, TailReceiver)>
    for (HeadSender, TailSender)
where
    HeadSender: SenderTo<HeadReceiver>,
    HeadReceiver: ReceiverFrom<HeadSender>,
    (HeadSender::Execution, TailSender::ExecutionList): TupleList,
    TailSender: SenderListTo<TailReceiver>,
    (HeadSender, TailSender): SenderList,
    TailReceiver: TupleList,
    (HeadReceiver, TailReceiver): TupleList,
{
    type ExecutionList = (HeadSender::Execution, TailSender::ExecutionList);

    fn connect_list(self, (headr, tailr): (HeadReceiver, TailReceiver)) -> Self::ExecutionList {
        let heade = self.0.connect(headr);
        let taile = self.1.connect_list(tailr);
        (heade, taile)
    }
}

pub trait ExecutionList: TupleList {
    fn execute_list(self);
}

impl ExecutionList for () {
    fn execute_list(self) {}
}

impl<Head, Tail> ExecutionList for (Head, Tail)
where
    Head: Execution,
    Tail: ExecutionList,
    (Head, Tail): TupleList,
{
    fn execute_list(self) {
        self.0.execute();
        self.1.execute_list()
    }
}
