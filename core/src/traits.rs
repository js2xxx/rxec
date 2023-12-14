#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::future::Future;

pub trait Receiver<T> {
    fn receive(self, value: T);
}

impl<F, T> Receiver<T> for F
where
    F: FnOnce(T),
{
    fn receive(self, value: T) {
        self(value);
    }
}

pub trait ReceiverFrom<S: Sender + ?Sized>: Receiver<S::Output> {}

impl<S, R> ReceiverFrom<S> for R
where
    S: Sender + ?Sized,
    R: Receiver<S::Output>,
{
}

pub trait Sender {
    type Output;
}

impl<F: Future> Sender for F {
    type Output = <F as Future>::Output;
}

pub trait SenderTo<R>: Sender
where
    R: ReceiverFrom<Self>,
{
    type Execution: Execution;

    fn connect(self, receiver: R) -> Self::Execution;
}

pub trait Execution {
    fn execute(self);
}

impl Execution for () {
    fn execute(self) {}
}

impl<F: FnOnce()> Execution for F {
    fn execute(self) {
        self()
    }
}

impl<T: Execution, const N: usize> Execution for [T; N] {
    fn execute(self) {
        self.into_iter().for_each(|e| e.execute())
    }
}

#[cfg(feature = "alloc")]
impl<T: Execution> Execution for Vec<T> {
    fn execute(self) {
        self.into_iter().for_each(|e| e.execute())
    }
}

macro_rules! impl_exec_for_tuples {
    () => ();
    ($head:ident, $($tail:ident,)*) => {
        impl_exec_for_tuples!(@IMPL $head, $($tail,)*);
        impl_exec_for_tuples!($($tail,)*);
    };
    (@IMPL $($ident:ident,)*) => {
        impl<$($ident: Execution),*> Execution for ($($ident,)*) {
            #[allow(non_snake_case)]
            fn execute(self) {
                let ($($ident,)*) = self;
                $($ident.execute();)*
            }
        }
    };
}

impl_exec_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L,);
