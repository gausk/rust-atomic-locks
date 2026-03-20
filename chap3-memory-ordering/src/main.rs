#![allow(static_mut_refs)]
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64};
use std::time::Duration;

fn main() {
    // 1. the basic happens-before rule is that everything that happens
    // within the same thread happens in order,
    // but for other thread operations might appear to happen in opposite order.

    static X: AtomicI32 = AtomicI32::new(0);
    static Y: AtomicI32 = AtomicI32::new(0);

    fn a() {
        X.store(10, Relaxed); // 1
        Y.store(20, Relaxed); // 2
    }

    fn b() {
        let y = Y.load(Relaxed); // 3
        let x = X.load(Relaxed); // 4
        println!("{x} {y}");
    }
    // for b, the output can also be 0 20

    // 2. Spawning a thread creates a happens-before relationship between what happened
    // before the spawn() call, and the new thread.
    // Similarly, joining a thread creates a happens-before relationship between the joined
    // thread and what happens after the join() call

    static Z: AtomicI32 = AtomicI32::new(0);

    fn f() {
        let x = Z.load(Relaxed);
        assert!(x == 1 || x == 2);
    }
    Z.store(1, Relaxed);
    let t = std::thread::spawn(f);
    Z.store(2, Relaxed);
    t.join().unwrap();
    Z.store(3, Relaxed);

    // Relaxed Ordering

    // do not provide any happens-before relationship
    // do guarantee a total modification order of each individual atomic variable
    // all modifications of the same atomic variable happen in an order that is the
    // same from the perspective of every single thread.

    static R: AtomicI32 = AtomicI32::new(0);

    fn ra() {
        R.fetch_add(5, Relaxed);
        R.fetch_add(10, Relaxed);
    }

    fn rb() {
        let a = R.load(Relaxed);
        let b = R.load(Relaxed);
        let c = R.load(Relaxed);
        let d = R.load(Relaxed);
        println!("{a} {b} {c} {d}");
    }

    // "0 0 0 0", "0 0 5 15", and "0 15 15 15" are some of the possible results
    // from the print statement in the other thread,
    // while an output of "0 5 0 15" or "0 0 10 15" is impossible

    fn a1() {
        R.fetch_add(5, Relaxed);
    }

    fn a2() {
        R.fetch_add(10, Relaxed);
    }
    // two possible modification orders: either 0→5→15, or 0→10→15

    // Out of Thin Air Values
    // out-of-thin-air values is universally considered to be a bug in the theoretical model

    static XA: AtomicI32 = AtomicI32::new(0);
    static YA: AtomicI32 = AtomicI32::new(0);

    fn thin_air() {
        let a = std::thread::spawn(|| {
            let x = XA.load(Relaxed);
            YA.store(x, Relaxed);
        });
        let b = std::thread::spawn(|| {
            let y = YA.load(Relaxed);
            XA.store(y, Relaxed);
        });
        a.join().unwrap();
        b.join().unwrap();
        assert_eq!(X.load(Relaxed), 0); // Might fail?
        assert_eq!(Y.load(Relaxed), 0); // Might fail?
    }

    // Release and Acquire Ordering

    // Release and acquire memory ordering are used in a pair to form a happens-before
    // relationship between threads.
    // Release memory ordering applies to store operations, while
    // Acquire memory ordering applies to load operations
    // AcqRel is used to represent the combination of Acquire and Release,
    // which causes both the load to use acquire ordering, and the store to use release ordering

    // A happens-before relationship is formed when an acquire-load operation
    // observes the result of a release-store operation.
    // In this case, the store and everything before it,
    // happened before the load and everything after it.

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

    /*
    static mut DATA: u64 = 0;
    static READY: AtomicBool = AtomicBool::new(false);

    fn acquire_and_release() {
        std::thread::spawn(|| {
            // Safety: Nothing else is accessing DATA,
            // because we haven't set the READY flag yet.
            unsafe { DATA = 123 };
            READY.store(true, Release); // Everything from before this store ..
        });
        while !READY.load(Acquire) { // .. is visible after this loads `true`.
            std::thread::sleep(Duration::from_millis(100));
            println!("waiting...");
        }
        // Safety: Nothing is mutating DATA, because READY is set.
        println!("{}", unsafe { DATA });
    }
     */

    static mut S: String = String::new();
    static LOCKED: AtomicBool = AtomicBool::new(false);

    fn lock(i: i32) {
        if LOCKED
            .compare_exchange(false, true, Acquire, Relaxed)
            .is_ok()
        {
            // Safety: We hold the exclusive lock, so nothing else is accessing DATA.
            unsafe { S.extend(i.to_string().chars()) };
            LOCKED.store(false, Release);
        }
    }

    fn locking() {
        std::thread::scope(|s| {
            for i in 0..100 {
                s.spawn(move || lock(i));
            }
        });
    }

    locking();
    println!("{:?}", unsafe { &S });

    // Sequentially Consistent Ordering

    // It includes all the guarantees of acquire ordering (for loads)
    // and release ordering (for stores),
    // and also guarantees a globally consistent order of operations

    use std::sync::atomic::Ordering::SeqCst;

    static A: AtomicBool = AtomicBool::new(false);
    static B: AtomicBool = AtomicBool::new(false);

    static mut SEQ: String = String::new();

    let a = std::thread::spawn(|| {
        A.store(true, SeqCst);
        if !B.load(SeqCst) {
            unsafe { SEQ.push('b') };
        }
    });

    let b = std::thread::spawn(|| {
        B.store(true, SeqCst);
        if !A.load(SeqCst) {
            unsafe { SEQ.push('a') };
        }
    });

    a.join().unwrap();
    b.join().unwrap();
    println!("{:?}", unsafe { &SEQ });
}
