use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, Ordering};

use atomic_wait::{wait, wake_all, wake_one};

pub struct RwLock<T> {
    data: UnsafeCell<T>,
    writer_wake_counter: AtomicU32,
    locked: AtomicU32,
}

unsafe impl<T> Sync for RwLock<T> where T: Send {}

pub struct ReadGuard<'a, T> {
    pub(crate) lock: &'a RwLock<T>,
}

impl<'a, T> Drop for ReadGuard<'a, T> {
    fn drop(&mut self) {
        if self.lock.locked.fetch_sub(1, Ordering::Release) == 1 {
            self.lock
                .writer_wake_counter
                .fetch_add(1, Ordering::Release);
            wake_one(&self.lock.locked);
        }
    }
}

impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // Safety: The very existence of this Guard
        // guarantees we've exclusively locked the lock.
        unsafe { &*self.lock.data.get() }
    }
}

pub struct WriteGuard<'a, T> {
    pub(crate) lock: &'a RwLock<T>,
}

impl<'a, T> Drop for WriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(0, Ordering::Release);
        self.lock
            .writer_wake_counter
            .fetch_add(1, Ordering::Release);
        // Wake up all waiting readers and writers.
        wake_one(&self.lock.writer_wake_counter);
        wake_all(&self.lock.locked);
    }
}

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // Safety: The very existence of this Guard
        // guarantees we've exclusively locked the lock.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: The very existence of this Guard
        // guarantees we've exclusively locked the lock.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> RwLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            locked: AtomicU32::new(0),
            writer_wake_counter: AtomicU32::new(0),
        }
    }

    pub fn read_lock(&self) -> ReadGuard<'_, T> {
        let mut state = self.locked.load(Ordering::Relaxed);
        loop {
            if state < u32::MAX {
                match self.locked.compare_exchange_weak(
                    state,
                    state + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(e) => state = e,
                }
            } else {
                wait(&self.locked, u32::MAX);
                state = self.locked.load(Ordering::Relaxed);
            }
        }
        ReadGuard { lock: self }
    }

    pub fn write_lock(&self) -> WriteGuard<'_, T> {
        while self
            .locked
            .compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            let w = self.writer_wake_counter.load(Ordering::Acquire);
            if self.locked.load(Ordering::Relaxed) != 0 {
                wait(&self.writer_wake_counter, w);
            }
        }
        WriteGuard { lock: self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rw_lock() {
        let x = RwLock::new(Vec::new());
        std::thread::scope(|s| {
            s.spawn(|| x.write_lock().push(1));
            s.spawn(|| {
                let mut g = x.write_lock();
                g.push(2);
                g.push(2);
            });
        });
        let g = x.read_lock();
        assert!(g.as_slice() == [1, 2, 2] || g.as_slice() == [2, 2, 1]);
    }
}
