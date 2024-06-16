use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project::pin_project;
use rxec_core::{Execution, Receiver, SenderTo};

#[cfg(feature = "std")]
pub fn wait<T, S: SenderTo<Recv<T>, Output = T>>(s: S) -> T {
    let (tx, rx) = oneshot::channel();
    s.connect(Recv(tx)).execute();
    rx.recv().expect("The task has been canceled")
}

#[derive(Debug)]
#[pin_project]
pub struct Async<T, S>
where
    S: SenderTo<Recv<T>, Output = T>,
{
    exec: Option<S::Execution>,
    #[pin]
    rx: oneshot::Receiver<S::Output>,
}

impl<T, S> Future for Async<T, S>
where
    S: SenderTo<Recv<T>, Output = T>,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let this = self.project();
        if let Some(execution) = this.exec.take() {
            execution.execute()
        }
        match this.rx.poll(cx) {
            Poll::Ready(Ok(value)) => Poll::Ready(value),
            Poll::Ready(Err(_)) => panic!("The task has been canceled"),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T, S> From<S> for Async<T, S>
where
    S: SenderTo<Recv<T>, Output = T>,
{
    fn from(s: S) -> Self {
        let (tx, rx) = oneshot::channel();
        Async {
            exec: Some(s.connect(Recv(tx))),
            rx,
        }
    }
}

impl<T, S> Async<T, S>
where
    S: SenderTo<Recv<T>, Output = T>,
{
    pub fn new(s: S) -> Self {
        Async::from(s)
    }
}

#[derive(Debug)]
pub struct Recv<T>(oneshot::Sender<T>);

impl<T> Receiver<T> for Recv<T> {
    fn receive(self, value: T) {
        let _ = self.0.send(value);
    }
}
