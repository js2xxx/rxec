mod bind;
mod eager;
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

use rxec_core::Sender;

pub use self::{
    bind::{Bind, bind},
    eager::{Eager, eager},
    map::{Map, map},
    sched::{Scheduler, schedule},
    sched_on::{SchedOn, sched_on},
    transfer::{Transfer, transfer},
    value::{Value, value},
};
#[cfg(feature = "alloc")]
pub use self::{
    join_all::{JoinAll, JoinAllExt, join_all},
    join_tuple::{JoinTuple, JoinTupleExt, join, join_tuple},
    select::{Select, select},
    wait::Async,
};
#[cfg(feature = "std")]
pub use self::{
    sched::{ArcLoop, ArcLoopExec, ArcLoopSender, Loop, LoopExec, LoopSender},
    wait::wait,
};

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
    fn join<T>(self, other: T) -> JoinTuple<(Self, T)>
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
    {$e:expr} => ($crate::sender::value($e));
    {let $v:pat = @ $e:expr $(=> $ty:ty)?; $($t:tt)*} => {{
        let closure = move |$v $(:$ty)?| $crate::exec!($($t)*);
        $crate::sender::bind($e, closure)
    }};
    {let $v:pat = $e:expr $(=> $ty:ty)?; $($t:tt)*} => {{
        let closure = move |$v $(:$ty)?| $crate::exec!($($t)*);
        $crate::sender::bind($crate::sender::value($e), closure)
    }};
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use std::{print, println};

    use super::{Loop, Scheduler, SenderExt, value, wait};

    #[test]
    fn basic() {
        let rl = Loop::new();

        let work1 = value(String::from("Hello"))
            .map(|s| print!("{s} "))
            .map(|_| 1)
            .sched_on(&rl);

        let work2 = exec! {
            let _ = @rl.schedule();
            let s = String::from("World");
            let _ = println!("{s}");
            2
        };

        let (r1, r2) = wait(work1.join(work2));
        assert_eq!((r1, r2), (1, 2));
    }
}
