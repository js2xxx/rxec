use core::{
    cell::UnsafeCell,
    convert::Infallible,
    marker::{PhantomData, PhantomPinned},
    pin::Pin,
};

use pin_project::pin_project;
use placid::{place::DynPlace, prelude::*};
use tsum::{Sum, T, t};

use crate::{OperationState, ReceiverFrom, Sender, SenderTo, basic::*};

pub struct AndThenExpr<S, F>(PhantomData<(S, F)>);

#[derive(InitPin)]
#[pin_project]
pub struct AndThenState<O, F> {
    #[pin]
    pinned: PhantomPinned,
    func: CountDownSlot<F>,
    #[pin]
    next_op: UnsafeCell<DynPlace<O>>,
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

    fn receiver_count_down(_: &Self::Data) -> usize {
        1
    }
}

impl<S, F, T, R> SenderExprTo<R> for AndThenExpr<S, F>
where
    S: Sender,
    F: FnOnce(S::Output) -> T,
    T: SenderTo<R, Operation: Sized>,
    R: ReceiverFrom<T>,
{
    type State = AndThenState<T::Operation, F>;
    type Error = Infallible;
    type CreateState = impl InitPin<Self::State, Error = Self::Error>;

    fn create_state(data: Self::Data, _: &mut Self::SubSenders, _: &mut R) -> Self::CreateState {
        init_pin!(AndThenState {
            pinned: PhantomPinned,
            func: || CountDownSlot::new(1, data),
            next_op: || UnsafeCell::new(DynPlace::new()),
        })
    }

    fn complete(state: Pin<&BasicState<Self, R>>, value: Sum![S::Output]) {
        if let (Some(func), Some(recv)) = (state.state.func.take(), state.receiver.take()) {
            let next_sender = func(value.into_inner());
            let next_op = next_sender.connect(recv);

            // Create a dedicated function to ensure the lifetime is handled correctly.
            #[expect(clippy::mut_from_ref)]
            const unsafe fn unsafe_pin_get<T>(t: Pin<&UnsafeCell<T>>) -> Pin<&mut T> {
                // SAFETY: The caller must ensure that there are no concurrent accesses to the
                // UnsafeCell while the returned Pin is in use.
                unsafe { Pin::new_unchecked(&mut *t.get_ref().get()) }
            }

            // SAFETY: There's only 1 sub-sender, so no concurrent access.
            let place = unsafe { unsafe_pin_get(state.project_ref().state.project_ref().next_op) };
            if let Ok(next_op) = place.try_insert_pin(next_op) {
                next_op.start();
            }
        }
    }
}

pub type AndThen<'a, S, F> = BasicSender<'a, AndThenExpr<S, F>>;

pub const fn and_then<'a, S, F, T>(sender: S, func: F) -> AndThen<'a, S, F>
where
    S: Sender,
    F: FnOnce(S::Output) -> T,
    T: Sender,
{
    BasicSender::new(func, t![sender])
}
