use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, Ordering};

use atomic_wait::{wait, wake_one};

pub struct Mutex<T> {
    data: UnsafeCell<T>,
    locked: AtomicU32,
}

unsafe impl<T> Sync for Mutex<T> where T: Send {}

pub struct MutexGuard<'a, T> {
    pub(crate) lock: &'a Mutex<T>,
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        if self.lock.locked.swap(0, Ordering::Release) == 2 {
            wake_one(&self.lock.locked);
        }
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // Safety: The very existence of this Guard
        // guarantees we've exclusively locked the lock.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: The very existence of this Guard
        // guarantees we've exclusively locked the lock.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Mutex<T> {
        Self {
            data: UnsafeCell::new(data),
            locked: AtomicU32::default(),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        if self
            .locked
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // while self.locked.swap(2, Ordering::Acquire) != 0 {
            //     wait(&self.locked, 2);
            // }
            lock_contended(&self.locked);
        }
        MutexGuard { lock: self }
    }
}

#[cold]
fn lock_contended(state: &AtomicU32) {
    let mut spin_count = 0;

    while state.load(Ordering::Relaxed) == 1 && spin_count < 100 {
        spin_count += 1;
    }
    if state
        .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        return;
    }

    while state.swap(2, Ordering::Acquire) != 0 {
        wait(state, 2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutex() {
        let x = Mutex::new(Vec::new());
        std::thread::scope(|s| {
            s.spawn(|| x.lock().push(1));
            s.spawn(|| {
                let mut g = x.lock();
                g.push(2);
                g.push(2);
            });
        });
        let g = x.lock();
        assert!(g.as_slice() == [1, 2, 2] || g.as_slice() == [2, 2, 1]);
    }
}
