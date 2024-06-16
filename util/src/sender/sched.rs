use rxec_core::Sender;

pub trait Scheduler {
    type Sender: Sender<Output = ()>;

    fn schedule(self) -> Self::Sender;
}

pub fn schedule<S: Scheduler>(s: S) -> S::Sender {
    s.schedule()
}

#[cfg(feature = "std")]
mod run_loop {
    use alloc::{boxed::Box, collections::VecDeque, sync::Arc};
    use core::{
        iter,
        sync::atomic::{AtomicBool, Ordering::SeqCst},
    };
    use std::{
        sync::{Condvar, Mutex},
        thread::JoinHandle,
    };

    use rxec_core::{Execution, Receiver, ReceiverFrom, Sender, SenderTo};

    use super::Scheduler;

    type BoxedRecv = Box<dyn FnOnce() + Send>;

    struct Inner {
        data: Mutex<VecDeque<BoxedRecv>>,
        cv: Condvar,
        stopped: AtomicBool,
    }

    impl Inner {
        fn push(&self, r: BoxedRecv) {
            let mut data = self.data.lock().unwrap();
            data.push_back(r);
            std::println!("Runner push");
            self.cv.notify_one();
        }

        fn pop(&self) -> Option<BoxedRecv> {
            let mut data = self.data.lock().unwrap();
            while !self.stopped.load(SeqCst) {
                if let Some(r) = data.pop_front() {
                    std::println!("Runner pop");
                    return Some(r);
                }
                data = self.cv.wait(data).unwrap();
            }
            None
        }
    }

    pub struct Loop {
        inner: Arc<Inner>,
        thread: Option<JoinHandle<()>>,
    }

    impl Loop {
        pub fn new() -> Self {
            let inner = Arc::new(Inner {
                data: Mutex::new(VecDeque::new()),
                cv: Condvar::new(),
                stopped: AtomicBool::new(false),
            });
            let i2 = inner.clone();
            Loop {
                inner,
                thread: Some(std::thread::spawn(move || {
                    iter::from_fn(|| i2.pop()).for_each(|r| r());
                    std::println!("Runner exit");
                })),
            }
        }

        pub fn arc(&self) -> ArcLoop<'_> {
            ArcLoop(&self.inner)
        }
    }

    impl Default for Loop {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Drop for Loop {
        fn drop(&mut self) {
            self.inner.stopped.store(true, SeqCst);
            self.inner.cv.notify_all();
            if let Some(thread) = self.thread.take() {
                thread.join().unwrap();
            }
        }
    }

    pub struct LoopSender<'a>(&'a Inner);

    pub struct LoopExec<'a, R> {
        inner: &'a Inner,
        recv: R,
    }

    impl<'a> Scheduler for &'a Loop {
        type Sender = LoopSender<'a>;

        fn schedule(self) -> Self::Sender {
            LoopSender(&self.inner)
        }
    }

    impl<'a> Sender for LoopSender<'a> {
        type Output = ();
    }

    impl<'a, R> SenderTo<R> for LoopSender<'a>
    where
        R: ReceiverFrom<Self> + Send + 'static,
    {
        type Execution = LoopExec<'a, R>;

        fn connect(self, receiver: R) -> Self::Execution {
            LoopExec {
                inner: self.0,
                recv: receiver,
            }
        }
    }

    impl<'a, R> Execution for LoopExec<'a, R>
    where
        R: Receiver<()> + Send + 'static,
    {
        fn execute(self) {
            self.inner.push(Box::new(move || self.recv.receive(())))
        }
    }

    #[derive(Clone, Copy)]
    pub struct ArcLoop<'a>(&'a Arc<Inner>);

    pub struct ArcLoopSender(Arc<Inner>);

    pub struct ArcLoopExec<R> {
        inner: Arc<Inner>,
        recv: R,
    }

    impl<'a> Scheduler for ArcLoop<'a> {
        type Sender = ArcLoopSender;

        fn schedule(self) -> Self::Sender {
            ArcLoopSender(self.0.clone())
        }
    }

    impl Sender for ArcLoopSender {
        type Output = ();
    }

    impl<R> SenderTo<R> for ArcLoopSender
    where
        R: ReceiverFrom<Self> + Send + 'static,
    {
        type Execution = ArcLoopExec<R>;

        fn connect(self, receiver: R) -> Self::Execution {
            ArcLoopExec {
                inner: self.0,
                recv: receiver,
            }
        }
    }

    impl<R> Execution for ArcLoopExec<R>
    where
        R: Receiver<()> + Send + 'static,
    {
        fn execute(self) {
            self.inner.push(Box::new(move || self.recv.receive(())))
        }
    }
}
#[cfg(feature = "std")]
pub use self::run_loop::*;
