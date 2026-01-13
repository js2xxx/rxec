use core::{
    fmt,
    marker::{PhantomData, PhantomPinned},
    mem::{self, ManuallyDrop},
    pin::Pin,
};

use pin_project::pin_project;
use placid::{
    init::InitPinError,
    pin::{DropSlot, DroppingSlot},
    prelude::*,
};
use tsum::{
    Sum,
    sum::{
        index::{UInt, UTerm},
        repr,
    },
};

use crate::{
    basic::arc::{ListPlaceRef, ListPlaceT},
    list::{
        CountListT, IndexList, IndexListT, OperationStateList, PCons, PTerm, SenderList,
        SenderOutputList, UIndex, USub, USubT,
    },
    traits::{ConnectOp, OperationState, Receiver, Sender, SenderOutput, SenderTo},
};

mod slot;
pub use self::slot::CountDownSlot;

mod arc;
pub use self::arc::{ListPlace, PArc};

pub trait SenderExpr: Sized {
    type Output;
    type Data;
    type SubSenders: SenderList + ListPlace;

    fn receiver_count_down(data: &Self::Data) -> usize;
}

pub trait SenderExprTo<R>: SenderExpr {
    type State;
    type Error: fmt::Debug;

    type CreateState: InitPin<Self::State, Error = Self::Error>;

    fn create_state(
        data: Self::Data,
        sub_senders: &mut Self::SubSenders,
        receiver: &mut R,
    ) -> Self::CreateState;

    fn start<'a>(state: StateRef<'_, Self, R>, subops: Pin<&mut ConnectAllOps<'a, Self, R>>)
    where
        BasicState<Self, R>: ConnectAll<'a, Self, R>,
    {
        let _ = state;
        subops.start_all();
    }

    fn complete(state: StateRef<'_, Self, R>, value: Sum<SenderOutputList<Self::SubSenders>>);
}
pub type StatePlace<S, R> = ListPlaceT<<S as SenderExpr>::SubSenders, BasicState<S, R>>;
pub type StateRef<'a, S, R> = ListPlaceRef<'a, <S as SenderExpr>::SubSenders, BasicState<S, R>>;

pub struct BasicReceiver<'a, S, R, U: UIndex>
where
    S: SenderExprTo<R> + 'a,
    R: 'a,
{
    index: PhantomData<U>,
    state: StateRef<'a, S, R>,
}

impl<S, R, U> Receiver<SenderOutput<IndexListT<S::SubSenders, U>>> for BasicReceiver<'_, S, R, U>
where
    S: SenderExprTo<R, SubSenders: IndexList<U, Output: Sender>>,
    SenderOutputList<S::SubSenders>: repr::Split<SenderOutput<IndexListT<S::SubSenders, U>>, U>,
    U: UIndex,
{
    fn set(self, value: SenderOutput<IndexListT<S::SubSenders, U>>) {
        S::complete(self.state, Sum::new(value));
    }
}

#[derive(InitPin)]
#[pin_project]
pub struct BasicState<S, R>
where
    S: SenderExprTo<R>,
{
    #[pin]
    _marker: PhantomPinned,
    #[pin]
    pub state: S::State,
    pub receiver: CountDownSlot<R>,
}

pub trait ConnectList<'a, P: 'a, SubSenders: SenderList, E, U: UIndex> {
    type Operations: OperationStateList;

    fn connect_list(this: P, sub_senders: SubSenders) -> impl InitPin<Self::Operations, Error = E>;
}

impl<'a, S, R, E: fmt::Debug> ConnectList<'a, Pin<&'a mut Self>, (), E, UTerm> for BasicState<S, R>
where
    S: SenderExprTo<R>,
{
    type Operations = PTerm;

    fn connect_list(
        _this: Pin<&'a mut Self>,
        _sub_senders: (),
    ) -> impl InitPin<Self::Operations, Error = E> {
        init_pin!(PTerm)
    }
}

impl<'a, S, R, T, E: fmt::Debug> ConnectList<'a, Pin<&'a mut Self>, (T, ()), E, UInt<UTerm>>
    for BasicState<S, R>
where
    S: SenderExprTo<R, SubSenders: ListPlace<Ref<'a, Self> = Pin<&'a mut Self>>> + 'a,
    R: 'a,
    // Head bounds
    CountListT<S::SubSenders>: USub<UInt<UTerm>>,
    S::SubSenders: IndexList<USubT<CountListT<S::SubSenders>, UInt<UTerm>>, Output = T>,
    SenderOutputList<S::SubSenders>:
        repr::Split<SenderOutput<T>, USubT<CountListT<S::SubSenders>, UInt<UTerm>>>,
    T: SenderTo<
            BasicReceiver<'a, S, R, USubT<CountListT<S::SubSenders>, UInt<UTerm>>>,
            ConnectError: Into<E>,
        >,
{
    type Operations = PCons<
        ConnectOp<T, BasicReceiver<'a, S, R, USubT<CountListT<S::SubSenders>, UInt<UTerm>>>>,
        PTerm,
    >;

    fn connect_list(
        state: Pin<&'a mut Self>,
        sub_senders: (T, ()),
    ) -> impl InitPin<Self::Operations, Error = E> {
        let head_receiver = BasicReceiver { index: PhantomData, state };
        let head_operation = sub_senders.0.connect(head_receiver);
        init_pin!(PCons(
            head_operation.map_err(Into::into),
            #[pin]
            PTerm
        ))
    }
}

impl<'a, S, R, E: fmt::Debug> ConnectList<'a, PArc<'a, Self>, (), E, UTerm> for BasicState<S, R>
where
    S: SenderExprTo<R>,
{
    type Operations = PTerm;

    fn connect_list(
        _this: PArc<'a, Self>,
        _sub_senders: (),
    ) -> impl InitPin<Self::Operations, Error = E> {
        init_pin!(PTerm)
    }
}

impl<'a, S, R, Head, Tail, U, E> ConnectList<'a, PArc<'a, Self>, (Head, Tail), E, UInt<U>>
    for BasicState<S, R>
where
    S: SenderExprTo<R, SubSenders: ListPlace<Ref<'a, Self> = PArc<'a, Self>>> + 'a,
    R: 'a,
    // Head bounds
    CountListT<S::SubSenders>: USub<UInt<U>>,
    S::SubSenders: IndexList<USubT<CountListT<S::SubSenders>, UInt<U>>, Output = Head>,
    SenderOutputList<S::SubSenders>:
        repr::Split<SenderOutput<Head>, USubT<CountListT<S::SubSenders>, UInt<U>>>,
    Head: SenderTo<
            BasicReceiver<'a, S, R, USubT<CountListT<S::SubSenders>, UInt<U>>>,
            ConnectError: Into<E>,
        >,
    // Tail bounds
    Tail: SenderList,
    Self: ConnectList<'a, PArc<'a, Self>, Tail, E, U>,
    // Other bounds
    U: UIndex,
    E: fmt::Debug,
{
    type Operations = PCons<
        ConnectOp<Head, BasicReceiver<'a, S, R, USubT<CountListT<S::SubSenders>, UInt<U>>>>,
        <Self as ConnectList<'a, PArc<'a, Self>, Tail, E, U>>::Operations,
    >;

    fn connect_list(
        this: PArc<'a, Self>,
        (head, tail): (Head, Tail),
    ) -> impl InitPin<Self::Operations, Error = E> {
        let head_receiver = BasicReceiver {
            index: PhantomData,
            state: this.clone(),
        };
        let head_operation = head.connect(head_receiver);
        let tail_operations = Self::connect_list(this, tail);
        init_pin!(PCons(head_operation.map_err(Into::into), tail_operations))
    }
}

pub trait ConnectAll<'a, S: SenderExprTo<R> + 'a, R: 'a>:
    ConnectList<'a, StateRef<'a, S, R>, S::SubSenders, S::Error, CountListT<S::SubSenders>>
{
    fn connect_all(
        this: StateRef<'a, S, R>,
        sub_senders: S::SubSenders,
    ) -> impl InitPin<Self::Operations, Error = S::Error> {
        Self::connect_list(this, sub_senders)
    }
}
impl<'a, S, R, T> ConnectAll<'a, S, R> for T
where
    S: SenderExprTo<R> + 'a,
    R: 'a,
    T: ConnectList<'a, StateRef<'a, S, R>, S::SubSenders, S::Error, CountListT<S::SubSenders>>,
{
}

pub type ConnectAllOps<'a, S, R> = <BasicState<S, R> as ConnectList<
    'a,
    StateRef<'a, S, R>,
    <S as SenderExpr>::SubSenders,
    <S as SenderExprTo<R>>::Error,
    CountListT<<S as SenderExpr>::SubSenders>,
>>::Operations;

impl<S, R> BasicState<S, R>
where
    S: SenderExprTo<R>,
{
    fn new(
        data: S::Data,
        sub_senders: &mut S::SubSenders,
        mut receiver: R,
    ) -> impl InitPin<Self, Error = S::Error> {
        let count = S::receiver_count_down(&data);
        init_pin!(BasicState {
            _marker: PhantomPinned,
            state: S::create_state(data, sub_senders, &mut receiver),
            receiver: init::with(|| CountDownSlot::new(count, receiver)).adapt_err(),
        })
    }
}

#[derive(InitPin)]
#[pin_project]
pub struct BasicOperation<'a, S, R>
where
    S: SenderExprTo<R> + 'a,
    R: 'a,
    BasicState<S, R>: ConnectAll<'a, S, R>,
{
    #[pin]
    _marker: PhantomPinned,
    // `sub_ops` must come before `state`, as required by the safe drop
    // order stated in the comment below. See
    // https://doc.rust-lang.org/stable/reference/destructors.html#r-destructors.operation
    // for more details.
    #[pin]
    sub_ops: ConnectAllOps<'a, S, R>,
    // `state` itself must not be accessed since creation.
    #[pin]
    state: StatePlace<S, R>,
}

impl<'a, S, R> BasicOperation<'a, S, R>
where
    S: SenderExprTo<R> + 'a,
    R: 'a,
    BasicState<S, R>: ConnectAll<'a, S, R>,
{
    pub fn new(
        data: S::Data,
        mut sub_senders: S::SubSenders,
        receiver: R,
    ) -> impl InitPin<Self, Error = S::Error> {
        init::try_raw_pin(move |mut uninit: Uninit<Self>, slot| unsafe {
            // SAFETY: Here we are creating potential self-referential structs:
            //
            // - `S, R: 'a` which implies `BasicState<S, R>: 'a`, which again implies
            //   `ConnectAllOps<'a, S, R>: 'a`;
            // - `BasicState<S, R>: ConnectAll<'a, S, R>` has a method that receives
            //   `Pin<&'a BasicState<S, R>>`, which could be then stored in
            //   `ConnectAllOps<'a, S, R>`;
            // - And we have `sub_ops` referencing `state`.
            //
            // However, this is safe as long as we never move the `BasicOperation` after
            // it has been pinned, and `sub_ops` drops before `state`.

            let ptr = uninit.as_mut_ptr();
            let state = &raw mut (*ptr).state;
            let sub_ops = &raw mut (*ptr).sub_ops;

            let mut subslot = ManuallyDrop::new(DroppingSlot::new());
            let subslot_ref = DropSlot::new_unchecked(&mut subslot);

            let value = BasicState::new(data, &mut sub_senders, receiver);
            let state_init = <S::SubSenders as ListPlace>::init_place(value);
            match Uninit::from_raw(state).try_write_pin(state_init, subslot_ref) {
                Ok(p) => mem::forget(p),
                Err(err) => return Err(InitPinError::new(err.error, uninit, slot)),
            }

            let mut subslot = ManuallyDrop::new(DroppingSlot::new());
            let subslot_ref = DropSlot::new_unchecked(&mut subslot);

            let borrow = <S::SubSenders as ListPlace>::move_out(Pin::new_unchecked(&mut *state));
            let ops_init =
                <BasicState<S, R> as ConnectAll<'a, S, R>>::connect_all(borrow, sub_senders);
            match Uninit::from_raw(sub_ops).try_write_pin(ops_init, subslot_ref) {
                Ok(p) => mem::forget(p),
                Err(err) => {
                    state.drop_in_place();
                    return Err(InitPinError::new(err.error, uninit, slot));
                }
            }

            Ok(uninit.assume_init_pin(slot))
        })
    }
}

impl<'a, S, R> OperationState for BasicOperation<'a, S, R>
where
    S: SenderExprTo<R>,
    BasicState<S, R>: ConnectAll<'a, S, R>,
    ConnectAllOps<'a, S, R>: OperationStateList,
{
    fn start(self: Pin<&mut Self>) {
        let this = self.project();
        let state = <S::SubSenders as ListPlace>::move_out(this.state);
        S::start(state, this.sub_ops);
    }
}

pub struct BasicSender<'a, S: SenderExpr> {
    data: S::Data,
    sub_senders: S::SubSenders,
    marker: PhantomData<&'a ()>,
}

impl<'a, S> BasicSender<'a, S>
where
    S: SenderExpr,
{
    pub const fn new(data: S::Data, sub_senders: S::SubSenders) -> Self {
        Self {
            data,
            sub_senders,
            marker: PhantomData,
        }
    }
}

impl<'a, S> Sender for BasicSender<'a, S>
where
    S: SenderExpr,
{
    type Output = S::Output;
}

impl<'a, S, R> SenderTo<R> for BasicSender<'a, S>
where
    S: SenderExprTo<R> + 'a,
    R: Receiver<S::Output> + 'a,
    BasicState<S, R>: ConnectAll<'a, S, R>,
{
    type Operation = BasicOperation<'a, S, R>;
    type ConnectError = S::Error;

    fn connect(self, receiver: R) -> impl InitPin<Self::Operation, Error = Self::ConnectError> {
        BasicOperation::new(self.data, self.sub_senders, receiver)
    }
}
