use either_slot::{
    tuple,
    tuple::{Concat, InElement, Whole},
};
use rxec_core::Receiver;
use tuple_list::{Tuple, TupleList};

pub trait ZipOption: TupleList {
    type Zipped: TupleList;

    fn zip_option(self) -> Option<Self::Zipped>;
}

impl ZipOption for () {
    type Zipped = ();

    fn zip_option(self) -> Option<Self::Zipped> {
        Some(())
    }
}

impl<Head, Tail> ZipOption for (Option<Head>, Tail)
where
    Tail: ZipOption,
    (Option<Head>, Tail): TupleList,
    (Head, Tail::Zipped): TupleList,
{
    type Zipped = (Head, Tail::Zipped);

    fn zip_option(self) -> Option<Self::Zipped> {
        self.0.zip(self.1.zip_option())
    }
}

pub trait ReceiverList: Tuple {
    fn consume(self);
}

macro_rules! impl_consume {
    () => {impl_consume!(@IMPL);};
    ($head:ident, $($ident:ident,)*) => {
        impl_consume!(@IMPL $head, $($ident,)*);
        impl_consume!($($ident,)*);
    };
    (@IMPL $($ident:ident,)*) => {
        impl<$($ident,)* R,> ReceiverList for ($($ident,)* R,)
        where
            R: Receiver<($($ident,)*)>
        {
            #[allow(non_snake_case)]
            fn consume(self) {
                let ($($ident,)* r,) = self;
                r.receive(($($ident,)*));
            }
        }
    }
}
impl_consume!(A, B, C, D, E, F, G, H, I, J, K,);

pub trait ConstructReceiver<R>: Tuple {
    type Receiver: TupleList;

    fn construct() -> (tuple::Sender<Self, R, ()>, Self::Receiver)
    where
        Self: Concat<(R,)>,
        <Self as Concat<(R,)>>::Output: Concat<()>,
        <Whole<Self, R, ()> as Tuple>::TupleList: InElement;
}

macro_rules! impl_construct_receiver {
    () => {
        impl_construct_receiver!(@IMPL);
    };
    ($head:ident, $($ident:ident,)*) => {
        impl_construct_receiver!(@IMPL $head, $($ident,)*);
        impl_construct_receiver!($($ident,)*);
    };
    (@IMPL $($ident:ident,)*) => {
        impl<$($ident,)* R,> ConstructReceiver<R> for ($($ident,)*) {
            type Receiver = impl_construct_receiver!(@DEF (), ($($ident,)*));

            #[allow(non_snake_case)]
            fn construct() -> (tuple::Sender<Self, R, ()>, Self::Receiver) {
                let ($($ident,)* rr,) = tuple::<($($ident,)* R,)>();
                (
                    rr,
                    impl_construct_receiver!(@INIT (), ($($ident,)*)),
                )
            }
        }
    };
    (@DEF ($($prefix:ident,)*), ($current:ident, $($suffix:ident,)*)) => {
        (
            super::Recv<R, ($($prefix,)*), $current, ($($suffix,)*)>,
            impl_construct_receiver!(@DEF ($($prefix,)* $current,), ($($suffix,)*)),
        )
    };
    (@DEF ($($prefix:ident,)*), ()) => (());
    (@INIT ($($prefix:ident,)*), ($current:ident, $($suffix:ident,)*)) => {
        (
            super::Recv { rr: $current },
            impl_construct_receiver!(@INIT ($($prefix,)* $current,), ($($suffix,)*)),
        )
    };
    (@INIT ($($prefix:ident,)*), ()) => (());
}
impl_construct_receiver!(A, B, C, D, E, F, G, H, I, J, K,);
