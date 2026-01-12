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

impl<R: Receiver<T>, T> SenderExprTo<R> for ValueExpr<T> {
    type State = CountDownSlot<T>;
    type Error = Infallible;
    type CreateState = impl InitPin<Self::State, Error = Self::Error>;

    fn create_state(data: Self::Data, _: &mut (), _: &mut R) -> Self::CreateState {
        init::with(move || CountDownSlot::new(1, data))
    }

    fn start<'a>(state: Pin<&BasicState<Self, R>>, _: Pin<&mut ConnectAllOps<'a, Self, R>>)
    where
        BasicState<Self, R>: ConnectAll<'a, Self, R>,
    {
        if let (Some(recv), Some(value)) = (state.receiver.take(), state.state.take()) {
            recv.set(value);
        }
    }

    fn complete(_: Pin<&BasicState<Self, R>>, value: tsum::Sum<()>) {
        value.unreachable();
    }
}

pub type Value<'a, T> = BasicSender<'a, ValueExpr<T>>;

pub const fn value<'a, T>(value: T) -> Value<'a, T> {
    BasicSender::new(value, ())
}
