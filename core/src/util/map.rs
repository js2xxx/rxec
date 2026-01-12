use core::{convert::Infallible, marker::PhantomData, pin::Pin};

use placid::prelude::*;
use tsum::{Sum, T, t};

use crate::{Receiver, Sender, basic::*};

pub struct MapExpr<S, F>(PhantomData<(S, F)>);

impl<S, F, T> SenderExpr for MapExpr<S, F>
where
    S: Sender,
    F: FnOnce(S::Output) -> T,
{
    type Output = T;

    type Data = F;

    type SubSenders = T![S];

    fn receiver_count_down(_: &Self::Data) -> usize {
        1
    }
}
impl<S, F, T, R> SenderExprTo<R> for MapExpr<S, F>
where
    F: FnOnce(S::Output) -> T,
    S: Sender,
    R: Receiver<T>,
{
    type State = CountDownSlot<F>;
    type Error = Infallible;
    type CreateState = impl InitPin<Self::State, Error = Self::Error>;

    fn create_state(data: Self::Data, _: &mut Self::SubSenders, _: &mut R) -> Self::CreateState {
        init::with(move || CountDownSlot::new(1, data))
    }

    fn complete(state: Pin<&BasicState<Self, R>>, value: Sum![S::Output]) {
        if let (Some(func), Some(recv)) = (state.state.take(), state.receiver.take()) {
            let result = func(value.into_inner());
            recv.set(result);
        }
    }
}

pub type Map<'a, S, F> = BasicSender<'a, MapExpr<S, F>>;

pub const fn map<'a, S, F, T>(sender: S, func: F) -> Map<'a, S, F>
where
    S: Sender,
    F: FnOnce(S::Output) -> T,
{
    BasicSender::new(func, t![sender])
}
