use core::{fmt, pin::Pin};

use placid::prelude::*;

pub trait ListPlace {
    type Place<T>;

    type Ref<'a, T>
    where
        T: 'a;

    fn borrow<'a, T>(place: Pin<&'a mut Self::Place<T>>) -> Self::Ref<'a, T>
    where
        T: 'a;

    fn init_place<T, I, M, E>(value: I) -> impl InitPin<Self::Place<T>, Error = E>
    where
        I: IntoInitPin<T, M, Error = E>,
        E: fmt::Debug;
}
pub type ListPlaceT<L, T> = <L as ListPlace>::Place<T>;
pub type ListPlaceRef<'a, L, T> = <L as ListPlace>::Ref<'a, T>;

impl ListPlace for () {
    type Place<T> = T;

    type Ref<'a, T>
        = Pin<&'a mut T>
    where
        T: 'a;

    fn borrow<'a, T>(place: Pin<&'a mut T>) -> Self::Ref<'a, T>
    where
        T: 'a,
    {
        place
    }

    fn init_place<T, I, M, E>(value: I) -> impl InitPin<Self::Place<T>, Error = E>
    where
        I: IntoInitPin<T, M, Error = E>,
        E: fmt::Debug,
    {
        value.into_init()
    }
}

impl<P> ListPlace for (P, ()) {
    type Place<T> = T;

    type Ref<'a, T>
        = Pin<&'a mut T>
    where
        T: 'a;

    fn borrow<'a, T>(place: Pin<&'a mut T>) -> Self::Ref<'a, T>
    where
        T: 'a,
    {
        place
    }

    fn init_place<T, I, M, E>(value: I) -> impl InitPin<Self::Place<T>, Error = E>
    where
        I: IntoInitPin<T, M, Error = E>,
        E: fmt::Debug,
    {
        value.into_init()
    }
}

impl<P, Head, Tail> ListPlace for (P, (Head, Tail))
where
    Tail: ListPlace,
{
    type Place<T> = T;

    type Ref<'a, T>
        = Pin<&'a T>
    where
        T: 'a;

    fn borrow<'a, T>(place: Pin<&'a mut Self::Place<T>>) -> Self::Ref<'a, T>
    where
        T: 'a,
    {
        place.into_ref()
    }

    fn init_place<T, I, M, E>(value: I) -> impl InitPin<Self::Place<T>, Error = E>
    where
        I: IntoInitPin<T, M, Error = E>,
        E: fmt::Debug,
    {
        value.into_init()
    }
}
