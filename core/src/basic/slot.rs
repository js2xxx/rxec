use core::{
    cell::UnsafeCell,
    mem::ManuallyDrop,
    sync::atomic::{AtomicIsize, Ordering::*},
};

pub struct CountDownSlot<T> {
    count: AtomicIsize,
    value: ManuallyDrop<UnsafeCell<T>>,
}

impl<T> CountDownSlot<T> {
    #[inline]
    pub fn new(count: usize, value: T) -> Self {
        Self {
            count: AtomicIsize::new(count.try_into().expect("too much count")),
            value: ManuallyDrop::new(UnsafeCell::new(value)),
        }
    }

    pub fn take(&self) -> Option<T> {
        if self.count.fetch_sub(1, AcqRel) == 1 {
            // SAFETY: We are the last to access the value.
            let value = unsafe { core::ptr::read(self.value.get()) };
            Some(value)
        } else {
            None
        }
    }
}

impl<T> Drop for CountDownSlot<T> {
    fn drop(&mut self) {
        if self.count.load(Acquire) > 0 {
            // SAFETY: We are dropping before all accesses are done.
            unsafe { ManuallyDrop::drop(&mut self.value) };
        }
    }
}
