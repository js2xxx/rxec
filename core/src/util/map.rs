use core::{convert::Infallible, marker::PhantomData, pin::Pin};

use placid::prelude::*;
use tsum::{Sum, T, t};

use crate::{Receiver, Sender, basic::*, util::ONESHOT_COMPLETED};

pub struct MapExpr<S, F>(PhantomData<(S, F)>);

impl<S, F, T> SenderExpr for MapExpr<S, F>
where
    S: Sender,
    F: FnOnce(S::Output) -> T,
{
    type Output = T;
    type Data = F;
    type SubSenders = T![S];
}

pub struct MapState<T>(Option<T>);

impl<F> Unpin for MapState<F> {}

impl<S, F, T, R> SenderExprTo<R> for MapExpr<S, F>
where
    F: FnOnce(S::Output) -> T,
    S: Sender,
    R: Receiver<T>,
{
    type State = MapState<(F, R)>;
    type Error = Infallible;
    type CreateState = impl InitPin<Self::State, Error = Self::Error>;

    fn create_state(data: Self::Data, _: &mut Self::SubSenders, recv: R) -> Self::CreateState {
        init::with(move || MapState(Some((data, recv))))
    }

    fn complete(state: Pin<&mut State<Self, R>>, value: Sum![S::Output]) {
        let (func, recv) = state.state_mut().0.take().expect(ONESHOT_COMPLETED);
        let result = func(value.into_inner());
        recv.set(result);
    }
}

pub type Map<S, F> = BasicSender<MapExpr<S, F>>;

pub const fn map<S, F, T>(sender: S, func: F) -> Map<S, F>
where
    S: Sender,
    F: FnOnce(S::Output) -> T,
{
    BasicSender::new(func, t![sender])
}
