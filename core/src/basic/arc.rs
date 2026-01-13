use core::{
    fmt,
    marker::{CoercePointee, PhantomPinned},
    mem::ManuallyDrop,
    ops::Deref,
    pin::Pin,
    sync::atomic::{self, AtomicUsize, Ordering::*},
};

use pin_project::{pin_project, pinned_drop};
use placid::prelude::*;

const REF_COUNT_MAX: usize = isize::MAX as usize;
#[cfg(target_pointer_width = "64")]
const REF_COUNT_SATURATED: usize = 0xC000_0000_0000_0000;
#[cfg(target_pointer_width = "32")]
const REF_COUNT_SATURATED: usize = 0xC000_0000;

#[derive(InitPin)]
#[pin_project(PinnedDrop)]
pub struct ArcPlace<T: ?Sized> {
    #[pin]
    _marker: PhantomPinned,
    cnt: AtomicUsize,
    #[pin]
    value: ManuallyDrop<T>,
}

#[derive(CoercePointee)]
#[repr(transparent)]
pub struct PArc<'a, T: ?Sized>(Pin<&'a mut ArcPlace<T>>);

impl<T: ?Sized> ArcPlace<T> {
    pub const fn new<I, M, E>(value: I) -> impl InitPin<Self, Error = E>
    where
        I: IntoInitPin<T, M, Error = E>,
        E: fmt::Debug,
    {
        init_pin!(ArcPlace {
            _marker: PhantomPinned,
            cnt: init::try_with(|| Ok(AtomicUsize::new(1))),
            #[pin]
            value: ManuallyDrop(value),
        })
    }
}

#[pinned_drop]
impl<T: ?Sized> PinnedDrop for ArcPlace<T> {
    fn drop(self: Pin<&mut Self>) {
        debug_assert!(
            self.cnt.load(Relaxed) > 0,
            "reference count underflow in `ArcPlace`"
        );
        atomic::fence(AcqRel);
        // We are here to drop the value, not `PArc`s.
        unsafe { ManuallyDrop::drop(Pin::into_inner_unchecked(self.project().value)) }
    }
}

impl<'a, T: ?Sized> PArc<'a, T> {
    pub fn new(place: Pin<&'a mut ArcPlace<T>>) -> Self {
        let cnt = place.cnt.fetch_add(1, Relaxed);
        if cnt >= REF_COUNT_MAX {
            place.cnt.store(REF_COUNT_SATURATED, Relaxed);
        }
        PArc(place)
    }

    pub fn as_ref(&self) -> Pin<&T> {
        let proj = self.0.as_ref().project_ref();
        // SAFETY: The value is pinned again.
        unsafe { proj.value.map_unchecked(|v| &**v) }
    }

    /// # Safety
    ///
    /// The caller must ensure that there are no concurrent accesses to the
    /// underlying value.
    pub unsafe fn as_mut_unchecked(&mut self) -> Pin<&mut T> {
        let proj = self.0.as_mut().project();
        // SAFETY: The value is pinned again.
        unsafe { proj.value.map_unchecked_mut(|v| &mut **v) }
    }

    pub fn as_mut(&mut self) -> Option<Pin<&mut T>> {
        if self.0.cnt.compare_exchange(2, 1, Acquire, Relaxed).is_err() {
            None
        } else {
            self.0.cnt.store(2, Release);
            // SAFETY: There are no concurrent accesses to the underlying value.
            Some(unsafe { self.as_mut_unchecked() })
        }
    }
}

impl<'a, T: ?Sized> Clone for PArc<'a, T> {
    fn clone(&self) -> Self {
        // SAFETY: `Pin<&mut ArcPlace<T>>` can be aliased thanks to `PhantomPinned`.
        PArc::new(unsafe { core::ptr::read(&self.0) })
    }
}

impl<'a, T: ?Sized> Drop for PArc<'a, T> {
    fn drop(&mut self) {
        let cnt = self.0.cnt.fetch_sub(1, Release);
        if cnt >= REF_COUNT_MAX {
            self.0.cnt.store(REF_COUNT_SATURATED, Relaxed);
        }
        debug_assert!(cnt > 1, "`ArcPlace` drops before `PArc`s");
    }
}

impl<'a, T: ?Sized> Deref for PArc<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref().get_ref()
    }
}

pub trait ListPlace {
    type Place<T>;

    type Ref<'a, T>
    where
        T: 'a;

    fn move_out<'a, T>(place: Pin<&'a mut Self::Place<T>>) -> Self::Ref<'a, T>
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

    fn move_out<'a, T>(place: Pin<&'a mut T>) -> Self::Ref<'a, T>
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

    fn move_out<'a, T>(place: Pin<&'a mut T>) -> Self::Ref<'a, T>
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
    type Place<T> = ArcPlace<T>;

    type Ref<'a, T>
        = PArc<'a, T>
    where
        T: 'a;

    fn move_out<'a, T>(place: Pin<&'a mut Self::Place<T>>) -> Self::Ref<'a, T>
    where
        T: 'a,
    {
        PArc::new(place)
    }

    fn init_place<T, I, M, E>(value: I) -> impl InitPin<Self::Place<T>, Error = E>
    where
        I: IntoInitPin<T, M, Error = E>,
        E: fmt::Debug,
    {
        ArcPlace::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t() {
        let mut place = pown!(ArcPlace::new(42u32));
        {
            let mut r = PArc::new(place.as_mut());
            *r.as_mut().unwrap() += 1;

            let mut r2 = r.clone();
            assert_eq!(*r, 43);
            assert_eq!(*r2, 43);

            drop(r);
            *r2.as_mut().unwrap() -= 1;
            assert_eq!(*r2, 42);
        }
    }
}
