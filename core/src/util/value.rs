use core::{convert::Infallible, marker::PhantomData, pin::Pin};

use placid::prelude::*;

use crate::{basic::*, traits::Receiver};

pub struct ValueExpr<T>(PhantomData<T>);

impl<T> SenderExpr for ValueExpr<T> {
    type Output = T;

    type Data = T;

    type SubSenders = ();

    fn receiver_count_down(_: &Self::Data) -> usize {
        1
    }
}

pub struct ValueState<T>(Option<T>);

impl<T> Unpin for ValueState<T> {}

impl<R: Receiver<T>, T> SenderExprTo<R> for ValueExpr<T> {
    type State = ValueState<(T, R)>;
    type Error = Infallible;
    type CreateState = impl InitPin<Self::State, Error = Self::Error>;

    fn create_state(data: Self::Data, _: &mut (), recv: R) -> Self::CreateState {
        init::value(ValueState(Some((data, recv))))
    }

    fn start(state: Pin<&mut State<Self, R>>, _: Pin<&mut ConnectAllOps<Self, R>>)
    where
        State<Self, R>: ConnectAll<Self, R>,
    {
        if let Some((value, recv)) = state.state_mut().0.take() {
            recv.set(value);
        }
    }

    fn complete(_: Pin<&mut State<Self, R>>, value: tsum::Sum<()>) {
        value.unreachable();
    }
}

pub type Value<T> = BasicSender<ValueExpr<T>>;

pub const fn value<T>(value: T) -> Value<T> {
    BasicSender::new(value, ())
}
