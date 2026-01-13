use core::{
    fmt,
    marker::{PhantomData, PhantomPinned},
    mem::{self, ManuallyDrop},
    pin::Pin,
    ptr::NonNull,
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

    fn start(state: StateRef<'_, Self, R>, subops: Pin<&mut ConnectAllOps<Self, R>>)
    where
        State<Self, R>: ConnectAll<Self, R>,
    {
        let _ = state;
        // SAFETY: Recursion invariant holds.
        unsafe { subops.start_list_by_ref() };
    }

    fn complete(state: StateRef<'_, Self, R>, value: Sum<SenderOutputList<Self::SubSenders>>);

    fn stop(state: StateRef<'_, Self, R>) {
        let _ = state;
    }
}
pub type StatePlace<S, R> = ListPlaceT<<S as SenderExpr>::SubSenders, State<S, R>>;
pub type StateRef<'a, S, R> = ListPlaceRef<'a, <S as SenderExpr>::SubSenders, State<S, R>>;

pub struct BasicReceiver<S, R, U: UIndex>
where
    S: SenderExprTo<R, SubSenders: IndexList<U, Output: Sender>>,
    SenderOutputList<S::SubSenders>: repr::Split<SenderOutput<IndexListT<S::SubSenders, U>>, U>,
    U: UIndex,
{
    index: PhantomData<U>,
    // Effectively a StateRef<'state, S, R>.
    state: NonNull<StatePlace<S, R>>,
}

unsafe impl<S, R, U> Send for BasicReceiver<S, R, U>
where
    S: SenderExprTo<R, SubSenders: IndexList<U, Output: Sender>>,
    SenderOutputList<S::SubSenders>: repr::Split<SenderOutput<IndexListT<S::SubSenders, U>>, U>,
    U: UIndex,
    for<'a> StateRef<'a, S, R>: Send,
{
}

unsafe impl<S, R, U> Sync for BasicReceiver<S, R, U>
where
    S: SenderExprTo<R, SubSenders: IndexList<U, Output: Sender>>,
    SenderOutputList<S::SubSenders>: repr::Split<SenderOutput<IndexListT<S::SubSenders, U>>, U>,
    U: UIndex,
    for<'a> StateRef<'a, S, R>: Sync,
{
}

impl<S, R, U> Receiver<SenderOutput<IndexListT<S::SubSenders, U>>> for BasicReceiver<S, R, U>
where
    S: SenderExprTo<R, SubSenders: IndexList<U, Output: Sender>>,
    SenderOutputList<S::SubSenders>: repr::Split<SenderOutput<IndexListT<S::SubSenders, U>>, U>,
    U: UIndex,
{
    fn set(self, value: SenderOutput<IndexListT<S::SubSenders, U>>) {
        S::complete(
            // SAFETY: The `state` is valid since this struct cannot escape the lifetime of
            // the `BasicOperation` that created it. The `State<S, R>` outlives this struct
            // since the `BasicReceiver` is only stored in the `sub_ops` field of the
            // `BasicOperation`, which is dropped before the `state` field, as ensured by
            // the safe drop order stated in the comment in `BasicOperation::new`.
            unsafe { <S::SubSenders as ListPlace>::from_raw(self.state) },
            Sum::new(value),
        );
    }
}

impl<S, R, U> Drop for BasicReceiver<S, R, U>
where
    S: SenderExprTo<R, SubSenders: IndexList<U, Output: Sender>>,
    SenderOutputList<S::SubSenders>: repr::Split<SenderOutput<IndexListT<S::SubSenders, U>>, U>,
    U: UIndex,
{
    fn drop(&mut self) {
        // SAFETY: See the safety comment in `<Self as Receiver<T>>::set`.
        S::stop(unsafe { <S::SubSenders as ListPlace>::from_raw(self.state) });
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

pub trait ConnectList<A: ListPlace, SubSenders: SenderList, E, U: UIndex>: Sized {
    type OpList: OperationStateList;

    fn connect_list(
        this: A::Ref<'_, Self>,
        sub_senders: SubSenders,
    ) -> impl InitPin<Self::OpList, Error = E>;
}

impl<S, R, A: ListPlace, E: fmt::Debug> ConnectList<A, (), E, UTerm> for State<S, R>
where
    S: SenderExprTo<R>,
{
    type OpList = PTerm;

    fn connect_list(
        _this: A::Ref<'_, Self>,
        _sub_senders: (),
    ) -> impl InitPin<Self::OpList, Error = E> {
        init_pin!(PTerm)
    }
}

impl<S, R, T, E: fmt::Debug> ConnectList<(T, ()), (T, ()), E, UInt<UTerm>> for State<S, R>
where
    S: SenderExprTo<R, SubSenders = (T, ())>,
    R:,
    // Head bounds
    T: SenderTo<BasicReceiver<S, R, UTerm>, ConnectError: Into<E>>,
{
    type OpList = PCons<ConnectOp<T, BasicReceiver<S, R, UTerm>>, PTerm>;

    fn connect_list<'a>(
        state: Pin<&mut Self>,
        sub_senders: (T, ()),
    ) -> impl InitPin<Self::OpList, Error = E> {
        let head_receiver = BasicReceiver {
            index: PhantomData,
            // SAFETY: We don't move `state` after this.
            state: NonNull::from_mut(unsafe { Pin::into_inner_unchecked(state) }),
        };
        let head_operation = sub_senders.0.connect(head_receiver);
        init_pin!(PCons(
            head_operation.map_err(Into::into),
            #[pin]
            PTerm
        ))
    }
}

impl<S, R, Head, Tail, U, E, T, THead, TTail>
    ConnectList<(T, (THead, TTail)), (Head, Tail), E, UInt<U>> for State<S, R>
where
    T: Sender,
    THead: Sender,
    TTail: ListPlace + SenderList,
    S: SenderExprTo<R, SubSenders = (T, (THead, TTail))>,
    // Head bounds
    CountListT<(T, (THead, TTail))>: USub<UInt<U>>,
    (T, (THead, TTail)): IndexList<USubT<CountListT<(T, (THead, TTail))>, UInt<U>>, Output = Head>,
    SenderOutputList<(T, (THead, TTail))>:
        repr::Split<SenderOutput<Head>, USubT<CountListT<(T, (THead, TTail))>, UInt<U>>>,
    Head: SenderTo<
            BasicReceiver<S, R, USubT<CountListT<(T, (THead, TTail))>, UInt<U>>>,
            ConnectError: Into<E>,
        >,
    // Tail bounds
    Tail: SenderList,
    Self: ConnectList<(T, (THead, TTail)), Tail, E, U>,
    // Other bounds
    U: UIndex,
    E: fmt::Debug,
{
    type OpList = PCons<
        ConnectOp<Head, BasicReceiver<S, R, USubT<CountListT<(T, (THead, TTail))>, UInt<U>>>>,
        <Self as ConnectList<(T, (THead, TTail)), Tail, E, U>>::OpList,
    >;

    fn connect_list<'a>(
        state: Pin<&Self>,
        (head, tail): (Head, Tail),
    ) -> impl InitPin<Self::OpList, Error = E> {
        let head_receiver = BasicReceiver {
            index: PhantomData,
            state: NonNull::from_ref(&*state),
        };
        let head_operation = head.connect(head_receiver);
        let tail_operations = Self::connect_list(state, tail);
        init_pin!(PCons(head_operation.map_err(Into::into), tail_operations))
    }
}

pub trait ConnectAll<S: SenderExprTo<R>, R>:
    ConnectList<S::SubSenders, S::SubSenders, S::Error, CountListT<S::SubSenders>>
{
    type Operations: OperationStateList;

    fn connect_all(
        this: ListPlaceRef<'_, S::SubSenders, Self>,
        sub_senders: S::SubSenders,
    ) -> impl InitPin<Self::Operations, Error = S::Error>;
}
impl<S, R, T> ConnectAll<S, R> for T
where
    S: SenderExprTo<R>,
    R:,
    T: ConnectList<S::SubSenders, S::SubSenders, S::Error, CountListT<S::SubSenders>>,
{
    type Operations = T::OpList;

    fn connect_all(
        this: ListPlaceRef<'_, S::SubSenders, T>,
        sub_senders: S::SubSenders,
    ) -> impl InitPin<Self::Operations, Error = S::Error> {
        Self::connect_list(this, sub_senders)
    }
}

pub type ConnectAllOps<S, R> = <State<S, R> as ConnectAll<S, R>>::Operations;

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
pub struct BasicOperation<S, R>
where
    S: SenderExprTo<R>,
    State<S, R>: ConnectAll<S, R>,
{
    #[pin]
    _marker: PhantomPinned,
    // `sub_ops` must come before `state`, as required by the safe drop
    // order stated in the comment below. See
    // https://doc.rust-lang.org/stable/reference/destructors.html#r-destructors.operation
    // for more details.
    #[pin]
    sub_ops: ConnectAllOps<S, R>,
    // `state` itself must not be accessed since creation.
    #[pin]
    state: StatePlace<S, R>,
}

impl<S, R> BasicOperation<S, R>
where
    S: SenderExprTo<R>,
    State<S, R>: ConnectAll<S, R>,
{
    pub fn new(
        data: S::Data,
        mut sub_senders: S::SubSenders,
        receiver: R,
    ) -> impl InitPin<Self, Error = S::Error> {
        init::try_raw_pin(move |mut uninit: Uninit<Self>, slot| unsafe {
            // SAFETY: Here we are creating potential self-referential structs: Any
            // `BasicReceiver<S, R>` that is stored in `sub_ops` contains a pointer to
            // `state`.
            //
            // However, this is safe as long as we never move the `BasicOperation` after
            // it has been pinned, and `state` outlives `sub_ops`, which means that
            // `sub_ops` must be dropped before `state`.

            let ptr = uninit.as_mut_ptr();
            let state = &raw mut (*ptr).state;
            let sub_ops = &raw mut (*ptr).sub_ops;
            {
                let mut subslot = ManuallyDrop::new(DroppingSlot::new());
                let subslot_ref = DropSlot::new_unchecked(&mut subslot);

                let value = State::new(data, &mut sub_senders, receiver);
                let state_init = <S::SubSenders as ListPlace>::init(value);
                match Uninit::from_raw(state).try_write_pin(state_init, subslot_ref) {
                    Ok(p) => mem::forget(p),
                    Err(err) => return Err(InitPinError::new(err.error, uninit, slot)),
                }
            }
            {
                let mut subslot = ManuallyDrop::new(DroppingSlot::new());
                let subslot_ref = DropSlot::new_unchecked(&mut subslot);

                let borrow = <S::SubSenders as ListPlace>::borrow(Pin::new_unchecked(&mut *state));
                let ops_init = <State<S, R> as ConnectAll<S, R>>::connect_all(borrow, sub_senders);
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

unsafe impl<S, R> OperationState for BasicOperation<S, R>
where
    S: SenderExprTo<R>,
    State<S, R>: ConnectAll<S, R>,
    ConnectAllOps<S, R>: OperationStateList,
{
    unsafe fn start_by_ref(self: Pin<&mut Self>) {
        let this = self.project();
        let state = <S::SubSenders as ListPlace>::borrow(this.state);
        S::start(state, this.sub_ops);
    }
}

pub struct BasicSender<S: SenderExpr> {
    data: S::Data,
    sub_senders: S::SubSenders,
}

impl<S> BasicSender<S>
where
    S: SenderExpr,
{
    pub const fn new(data: S::Data, sub_senders: S::SubSenders) -> Self {
        Self { data, sub_senders }
    }
}

impl<S> Sender for BasicSender<S>
where
    S: SenderExpr,
{
    type Output = S::Output;
}

impl<S, R> SenderTo<R> for BasicSender<S>
where
    S: SenderExprTo<R>,
    R: Receiver<S::Output>,
    State<S, R>: ConnectAll<S, R>,
{
    type Operation = BasicOperation<S, R>;
    type ConnectError = S::Error;

    fn connect(self, receiver: R) -> impl InitPin<Self::Operation, Error = Self::ConnectError> {
        BasicOperation::new(self.data, self.sub_senders, receiver)
    }
}
