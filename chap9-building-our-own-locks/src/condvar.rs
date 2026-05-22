use crate::mutex::MutexGuard;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicU32, AtomicUsize};

use atomic_wait::{wait, wake_all, wake_one};

pub struct CondVar {
    counter: AtomicU32,
    waiters: AtomicUsize,
}

impl CondVar {
    pub const fn new() -> Self {
        Self {
            counter: AtomicU32::new(0),
            waiters: AtomicUsize::new(0),
        }
    }

    pub fn notify_one(&self) {
        if self.waiters.load(Relaxed) > 0 {
            self.counter.fetch_add(1, Relaxed);
            wake_one(&self.counter);
        }
    }

    pub fn notify_all(&self) {
        if self.waiters.load(Relaxed) > 0 {
            self.counter.fetch_add(1, Relaxed);
            wake_all(&self.counter);
        }
    }

    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        let counter_value = self.counter.load(Relaxed);
        self.waiters.fetch_add(1, Relaxed); // doing this before dropping lock for happen before relationship
        let mutex = guard.lock;
        drop(guard);
        wait(&self.counter, counter_value);
        self.waiters.fetch_sub(1, Relaxed);
        mutex.lock()
    }
}

#[cfg(test)]
mod tests {
    use super::CondVar;
    use crate::mutex::Mutex;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_condvar() {
        let mutex = Mutex::new(0);
        let condvar = CondVar::new();

        let mut wakeups = 0;

        thread::scope(|s| {
            s.spawn(|| {
                thread::sleep(Duration::from_secs(1));
                *mutex.lock() = 123;
                condvar.notify_one();
            });

            let mut m = mutex.lock();
            while *m < 100 {
                m = condvar.wait(m);
                wakeups += 1;
            }

            assert_eq!(*m, 123);
        });

        // Check that the main thread actually did wait (not busy-loop),
        // while still allowing for a few spurious wake ups.
        assert!(wakeups < 5);
    }
}
