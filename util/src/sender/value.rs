use rxec_core::{Execution, Receiver, Sender, SenderTo};

pub fn value<T>(value: T) -> Value<T> {
    Value(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value<T>(T);

impl<T> Sender for Value<T> {
    type Output = T;
}

impl<T, R> SenderTo<R> for Value<T>
where
    R: Receiver<T>,
{
    type Execution = Exec<T, R>;

    fn connect(self, receiver: R) -> Self::Execution {
        Exec { value: self.0, receiver }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Exec<T, R> {
    value: T,
    receiver: R,
}

impl<T, R> Execution for Exec<T, R>
where
    R: Receiver<T>,
{
    fn execute(self) {
        self.receiver.receive(self.value);
    }
}
