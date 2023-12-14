mod bind;
mod eager;
#[cfg(feature = "alloc")]
mod join;
#[cfg(feature = "alloc")]
mod join_all;
#[cfg(feature = "alloc")]
mod join_tuple;
mod map;
mod sched;
mod sched_on;
#[cfg(feature = "alloc")]
mod select;
mod transfer;
mod value;
#[cfg(feature = "alloc")]
mod wait;

#[cfg(feature = "std")]
pub use self::wait::wait;
pub use self::{
    bind::{bind, Bind},
    eager::{eager, Eager},
    map::{map, Map},
    sched::{schedule, Scheduler},
    sched_on::{sched_on, SchedOn},
    transfer::{transfer, Transfer},
    value::{value, Value},
};
#[cfg(feature = "alloc")]
pub use self::{
    join::{join, Join},
    join_all::{join_all, JoinAll, JoinAllExt},
    join_tuple::{join_tuple, JoinTuple, JoinTupleExt},
    select::{select, Select},
    wait::Async,
};
use rxec_core::Sender;

pub trait SenderExt: Sender + Sized {
    fn bind<F, T>(self, f: F) -> Bind<Self, F>
    where
        F: FnOnce(Self::Output) -> T,
        T: Sender,
    {
        bind(self, f)
    }

    fn eager(self) -> Eager<Self> {
        eager(self)
    }

    fn map<F, T>(self, f: F) -> Map<Self, F>
    where
        F: FnOnce(Self::Output) -> T,
    {
        map(self, f)
    }

    fn sched_on<Sched>(self, sched: Sched) -> SchedOn<Self, Sched>
    where
        Sched: Scheduler,
    {
        sched_on(self, sched)
    }

    fn transfer<Sched>(self, sched: Sched) -> Transfer<Self, Sched>
    where
        Sched: Scheduler,
    {
        transfer(self, sched)
    }

    #[cfg(feature = "alloc")]
    fn join<T>(self, other: T) -> Join<Self, T>
    where
        T: Sender,
    {
        join(self, other)
    }

    #[cfg(feature = "alloc")]
    fn or<T>(self, other: T) -> Select<Self, T>
    where
        T: Sender,
    {
        select(self, other)
    }
}
impl<S: Sender> SenderExt for S {}

#[macro_export]
macro_rules! exec {
    {@ $e:expr} => ($e);
    {$e:expr} => ($crate::value($e));
    {let $v:pat = @ $e:expr $(=> $ty:ty)?; $($t:tt)*} => {{
        let closure = move |$v $(:$ty)?| $crate::exec!($($t)*);
        $crate::bind($e, closure)
    }};
    {let $v:pat = $e:expr $(=> $ty:ty)?; $($t:tt)*} => {{
        let closure = move |$v $(:$ty)?| $crate::exec!($($t)*);
        $crate::bind($crate::value($e), closure)
    }};
}
