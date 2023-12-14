use rxec_core::Sender;

pub trait Scheduler {
    type Sender: Sender<Output = ()>;

    fn schedule(self) -> Self::Sender;
}

pub fn schedule<S: Scheduler>(s: S) -> S::Sender {
    s.schedule()
}
