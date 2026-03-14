use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::time::Duration;

fn main() {
    use std::sync::atomic::Ordering::{Acquire, Release};

    static DATA: AtomicU64 = AtomicU64::new(0);
    static READY: AtomicBool = AtomicBool::new(false);

    std::thread::spawn(|| {
        DATA.store(123, Relaxed);
        READY.store(true, Release); // Everything from before this store ..
    });
    while !READY.load(Acquire) {
        // .. is visible after this loads `true`.
        std::thread::sleep(Duration::from_millis(100));
        println!("waiting...");
    }
    println!("{}", DATA.load(Relaxed));
}
