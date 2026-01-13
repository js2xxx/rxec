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
    basic::place::{ListPlaceRef, ListPlaceT},
    list::{
        CountListT, IndexList, IndexListT, OperationStateList, PCons, PTerm, SenderList,
        SenderOutputList, UIndex, USub, USubT,
    },
    traits::{ConnectOp, OperationState, Receiver, Sender, SenderOutput, SenderTo},
};

mod place;
pub use self::place::ListPlace;

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
        receiver: R,
    ) -> Self::CreateState;

    fn start<'a>(state: StateRef<'_, Self, R>, subops: Pin<&mut ConnectAllOps<'a, Self, R>>)
    where
        State<Self, R>: ConnectAll<'a, Self, R>,
    {
        let _ = state;
        subops.start_all();
    }

    fn complete(state: StateRef<'_, Self, R>, value: Sum<SenderOutputList<Self::SubSenders>>);
}
pub type StateRef<'a, S, R> = ListPlaceRef<'a, <S as SenderExpr>::SubSenders, State<S, R>>;

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
pub struct State<S, R>
where
    S: SenderExprTo<R>,
{
    #[pin]
    _marker: PhantomPinned,
    #[pin]
    state: S::State,
}

pub trait ConnectList<'a, P: 'a, SubSenders: SenderList, E, U: UIndex> {
    type OpList: OperationStateList;

    fn connect_list(this: P, sub_senders: SubSenders) -> impl InitPin<Self::OpList, Error = E>;
}

impl<'a, S, R, P: 'a, E: fmt::Debug> ConnectList<'a, P, (), E, UTerm> for State<S, R>
where
    S: SenderExprTo<R>,
{
    type OpList = PTerm;

    fn connect_list(_this: P, _sub_senders: ()) -> impl InitPin<Self::OpList, Error = E> {
        init_pin!(PTerm)
    }
}

impl<'a, S, R, T, E: fmt::Debug> ConnectList<'a, Pin<&'a mut Self>, (T, ()), E, UInt<UTerm>>
    for State<S, R>
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
    type OpList = PCons<
        ConnectOp<T, BasicReceiver<'a, S, R, USubT<CountListT<S::SubSenders>, UInt<UTerm>>>>,
        PTerm,
    >;

    fn connect_list(
        state: Pin<&'a mut Self>,
        sub_senders: (T, ()),
    ) -> impl InitPin<Self::OpList, Error = E> {
        let head_receiver = BasicReceiver { index: PhantomData, state };
        let head_operation = sub_senders.0.connect(head_receiver);
        init_pin!(PCons(
            head_operation.map_err(Into::into),
            #[pin]
            PTerm
        ))
    }
}

impl<'a, S, R, Head, Tail, U, E> ConnectList<'a, Pin<&'a Self>, (Head, Tail), E, UInt<U>>
    for State<S, R>
where
    S: SenderExprTo<R, SubSenders: ListPlace<Ref<'a, Self> = Pin<&'a Self>>> + 'a,
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
    Self: ConnectList<'a, Pin<&'a Self>, Tail, E, U>,
    // Other bounds
    U: UIndex,
    E: fmt::Debug,
{
    type OpList = PCons<
        ConnectOp<Head, BasicReceiver<'a, S, R, USubT<CountListT<S::SubSenders>, UInt<U>>>>,
        <Self as ConnectList<'a, Pin<&'a Self>, Tail, E, U>>::OpList,
    >;

    fn connect_list(
        state: Pin<&'a Self>,
        (head, tail): (Head, Tail),
    ) -> impl InitPin<Self::OpList, Error = E> {
        let head_receiver = BasicReceiver { index: PhantomData, state };
        let head_operation = head.connect(head_receiver);
        let tail_operations = Self::connect_list(state, tail);
        init_pin!(PCons(head_operation.map_err(Into::into), tail_operations))
    }
}

pub trait ConnectAll<'a, S: SenderExprTo<R> + 'a, R: 'a>:
    ConnectList<
        'a,
        ListPlaceRef<'a, S::SubSenders, State<S, R>>,
        S::SubSenders,
        S::Error,
        CountListT<S::SubSenders>,
    >
{
    type Operations: OperationStateList;

    fn connect_all(
        this: ListPlaceRef<'a, S::SubSenders, State<S, R>>,
        sub_senders: S::SubSenders,
    ) -> impl InitPin<Self::Operations, Error = S::Error>;
}
impl<'a, S, R, T> ConnectAll<'a, S, R> for T
where
    S: SenderExprTo<R> + 'a,
    R: 'a,
    T: ConnectList<
            'a,
            ListPlaceRef<'a, S::SubSenders, State<S, R>>,
            S::SubSenders,
            S::Error,
            CountListT<S::SubSenders>,
        >,
{
    type Operations = T::OpList;

    fn connect_all(
        this: ListPlaceRef<'a, S::SubSenders, State<S, R>>,
        sub_senders: S::SubSenders,
    ) -> impl InitPin<Self::Operations, Error = S::Error> {
        Self::connect_list(this, sub_senders)
    }
}

pub type ConnectAllOps<'a, S, R> = <State<S, R> as ConnectAll<'a, S, R>>::Operations;

impl<S, R> State<S, R>
where
    S: SenderExprTo<R>,
{
    pub(super) fn new(
        data: S::Data,
        sub_senders: &mut S::SubSenders,
        receiver: R,
    ) -> impl InitPin<Self, Error = S::Error> {
        init_pin!(State {
            _marker: PhantomPinned,
            state: S::create_state(data, sub_senders, receiver),
        })
    }

    pub fn state(self: Pin<&Self>) -> Pin<&S::State> {
        self.project_ref().state
    }

    pub fn state_mut(self: Pin<&mut Self>) -> Pin<&mut S::State> {
        self.project().state
    }
}

#[derive(InitPin)]
#[pin_project]
pub struct BasicOperation<'a, S, R>
where
    S: SenderExprTo<R> + 'a,
    R: 'a,
    State<S, R>: ConnectAll<'a, S, R>,
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
    state: ListPlaceT<S::SubSenders, State<S, R>>,
}

impl<'a, S, R> BasicOperation<'a, S, R>
where
    S: SenderExprTo<R> + 'a,
    R: 'a,
    State<S, R>: ConnectAll<'a, S, R>,
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
            {
                let mut subslot = ManuallyDrop::new(DroppingSlot::new());
                let subslot_ref = DropSlot::new_unchecked(&mut subslot);

                let value = State::new(data, &mut sub_senders, receiver);
                let state_init = <S::SubSenders as ListPlace>::init_place(value);
                match Uninit::from_raw(state).try_write_pin(state_init, subslot_ref) {
                    Ok(p) => mem::forget(p),
                    Err(err) => return Err(InitPinError::new(err.error, uninit, slot)),
                }
            }
            {
                let mut subslot = ManuallyDrop::new(DroppingSlot::new());
                let subslot_ref = DropSlot::new_unchecked(&mut subslot);

                let borrow = <S::SubSenders as ListPlace>::borrow(Pin::new_unchecked(&mut *state));
                let ops_init =
                    <State<S, R> as ConnectAll<'a, S, R>>::connect_all(borrow, sub_senders);
                match Uninit::from_raw(sub_ops).try_write_pin(ops_init, subslot_ref) {
                    Ok(p) => mem::forget(p),
                    Err(err) => {
                        state.drop_in_place();
                        return Err(InitPinError::new(err.error, uninit, slot));
                    }
                }
            }

            Ok(uninit.assume_init_pin(slot))
        })
    }
}

impl<'a, S, R> OperationState for BasicOperation<'a, S, R>
where
    S: SenderExprTo<R>,
    State<S, R>: ConnectAll<'a, S, R>,
    ConnectAllOps<'a, S, R>: OperationStateList,
{
    fn start(self: Pin<&mut Self>) {
        let this = self.project();
        let state = <S::SubSenders as ListPlace>::borrow(this.state);
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
    State<S, R>: ConnectAll<'a, S, R>,
{
    type Operation = BasicOperation<'a, S, R>;
    type ConnectError = S::Error;

    fn connect(self, receiver: R) -> impl InitPin<Self::Operation, Error = Self::ConnectError> {
        BasicOperation::new(self.data, self.sub_senders, receiver)
    }
}
