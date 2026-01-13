use core::{
    convert::Infallible,
    marker::{PhantomData, PhantomPinned},
    pin::Pin,
};

use pin_project::pin_project;
use placid::{place::DynPlace, prelude::*};
use tsum::{Sum, T, t};

use crate::{OperationState, ReceiverFrom, Sender, SenderTo, basic::*, util::ONESHOT_COMPLETED};

pub struct AndThenExpr<S, F>(PhantomData<(S, F)>);

#[derive(InitPin)]
#[pin_project]
pub struct AndThenState<O, F, R> {
    #[pin]
    pinned: PhantomPinned,
    data: Option<(F, R)>,
    #[pin]
    next_op: DynPlace<O>,
}

impl<S, F, T> SenderExpr for AndThenExpr<S, F>
where
    S: Sender,
    F: FnOnce(S::Output) -> T,
    T: Sender,
{
    type Output = T::Output;
    type Data = F;
    type SubSenders = T![S];
}

impl<S, F, T, R> SenderExprTo<R> for AndThenExpr<S, F>
where
    S: Sender,
    F: FnOnce(S::Output) -> T,
    T: SenderTo<R, Operation: Sized>,
    R: ReceiverFrom<T>,
{
    type State = AndThenState<T::Operation, F, R>;
    type Error = Infallible;
    type CreateState = impl InitPin<Self::State, Error = Self::Error>;

    fn create_state(data: Self::Data, _: &mut Self::SubSenders, recv: R) -> Self::CreateState {
        init_pin!(AndThenState {
            pinned: PhantomPinned,
            data: || Some((data, recv)),
            next_op: DynPlace::new,
        })
    }

    fn complete(state: Pin<&mut State<Self, R>>, value: Sum![S::Output]) {
        let state = state.state_mut().project();
        let (func, recv) = state.data.take().expect(ONESHOT_COMPLETED);
        let next_snd = func(value.into_inner());
        let next_op = next_snd.connect(recv);

        if let Ok(next_op) = state.next_op.try_insert_pin(next_op) {
            // SAFETY: The operation is started only once here, and the state is not
            // forgotten after started since it requires outer `OperationState::start`.
            unsafe { next_op.start_by_ref() };
        }
    }
}

pub type AndThen<S, F> = BasicSender<AndThenExpr<S, F>>;

pub const fn and_then<S, F, T>(sender: S, func: F) -> AndThen<S, F>
where
    S: Sender,
    F: FnOnce(S::Output) -> T,
    T: Sender,
{
    BasicSender::new(func, t![sender])
}
