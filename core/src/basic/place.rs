use core::{fmt, pin::Pin, ptr::NonNull};

use placid::prelude::*;

pub trait ListPlace {
    type Place<T>;

    type Ref<'a, T: 'a>;

    fn borrow<T>(place: Pin<&mut Self::Place<T>>) -> Self::Ref<'_, T>;

    unsafe fn from_raw<'a, T>(ptr: NonNull<Self::Place<T>>) -> Self::Ref<'a, T>;

    fn init<T, I, M, E>(value: I) -> impl InitPin<Self::Place<T>, Error = E>
    where
        I: IntoInitPin<T, M, Error = E>,
        E: fmt::Debug;
}
pub type ListPlaceT<L, T> = <L as ListPlace>::Place<T>;
pub type ListPlaceRef<'a, L, T> = <L as ListPlace>::Ref<'a, T>;

impl ListPlace for () {
    type Place<T> = T;

    type Ref<'a, T: 'a> = Pin<&'a mut T>;

    fn borrow<T>(place: Pin<&mut T>) -> Pin<&mut T> {
        place
    }

    unsafe fn from_raw<'a, T>(mut place: NonNull<T>) -> Pin<&'a mut T> {
        unsafe { Pin::new_unchecked(place.as_mut()) }
    }

    fn init<T, I, M, E>(value: I) -> impl InitPin<T, Error = E>
    where
        I: IntoInitPin<T, M, Error = E>,
        E: fmt::Debug,
    {
        value.into_init()
    }
}

impl<P> ListPlace for (P, ()) {
    type Place<T> = T;

    type Ref<'a, T: 'a> = Pin<&'a mut T>;

    fn borrow<T>(place: Pin<&mut T>) -> Pin<&mut T> {
        place
    }

    unsafe fn from_raw<'a, T>(mut place: NonNull<T>) -> Pin<&'a mut T> {
        unsafe { Pin::new_unchecked(place.as_mut()) }
    }

    fn init<T, I, M, E>(value: I) -> impl InitPin<Self::Place<T>, Error = E>
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

    type Ref<'a, T: 'a> = Pin<&'a T>;

    fn borrow<T>(place: Pin<&mut Self::Place<T>>) -> Pin<&T> {
        place.into_ref()
    }

    unsafe fn from_raw<'a, T>(ptr: NonNull<Self::Place<T>>) -> Pin<&'a T> {
        unsafe { Pin::new_unchecked(ptr.as_ref()) }
    }

    fn init<T, I, M, E>(value: I) -> impl InitPin<Self::Place<T>, Error = E>
    where
        I: IntoInitPin<T, M, Error = E>,
        E: fmt::Debug,
    {
        value.into_init()
    }
}
