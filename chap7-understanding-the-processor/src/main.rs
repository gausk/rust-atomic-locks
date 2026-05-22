use std::hint::black_box;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, compiler_fence};
use std::time::Instant;

/*
fn main() {
    static A: AtomicU64 = AtomicU64::new(0);

    let start = Instant::now();
    for _ in 0..1_000_000_000 {
        A.load(Relaxed);
    }
    println!("{:?}", start.elapsed());
}
*/

/*
static A: AtomicU64 = AtomicU64::new(0);

fn main() {
    black_box(&A); // New!
    let start = Instant::now();
    for _ in 0..1_000_000_000 {
        black_box(A.load(Relaxed)); // New!
    }
    println!("{:?}", start.elapsed());
}
*/

/*
static A: AtomicU64 = AtomicU64::new(0);

fn main() {
    black_box(&A);

    std::thread::spawn(|| {
        // New!
        loop {
            black_box(A.load(Relaxed));
        }
    });

    let start = Instant::now();
    for _ in 0..1_000_000_000 {
        black_box(A.load(Relaxed));
    }
    println!("{:?}", start.elapsed());
}
*/

/*
static A: AtomicU64 = AtomicU64::new(0);

fn main() {
    black_box(&A);

    std::thread::spawn(|| {
        // New!
        loop {
            black_box(A.store(0, Relaxed));
        }
    });

    let start = Instant::now();
    for _ in 0..1_000_000_000 {
        black_box(A.load(Relaxed));
    }
    println!("{:?}", start.elapsed());
}
*/

/*
static A: AtomicU64 = AtomicU64::new(0);

fn main() {
    black_box(&A);
    std::thread::spawn(|| {
        loop {
            black_box(A.compare_exchange(10, 20, Relaxed, Relaxed).is_ok()); // New!
        }
    });
    let start = Instant::now();
    for _ in 0..1_000_000_000 {
        black_box(A.load(Relaxed));
    }
    println!("{:?}", start.elapsed());
}
*/
/*
static A: [AtomicU64; 3] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

fn main() {
    black_box(&A);
    std::thread::spawn(|| {
        loop {
            A[0].store(0, Relaxed);
            A[2].store(0, Relaxed);
        }
    });
    let start = Instant::now();
    for _ in 0..1_000_000_000 {
        black_box(A[1].load(Relaxed));
    }
    println!("{:?}", start.elapsed());
}
*/

/*
#[repr(align(64))] // This struct must be 64-byte aligned.
struct Aligned(AtomicU64);

static A: [Aligned; 3] = [
    Aligned(AtomicU64::new(0)),
    Aligned(AtomicU64::new(0)),
    Aligned(AtomicU64::new(0)),
];

fn main() {
    black_box(&A);
    std::thread::spawn(|| {
        loop {
            A[0].0.store(1, Relaxed);
            A[2].0.store(1, Relaxed);
        }
    });
    let start = Instant::now();
    for _ in 0..1_000_000_000 {
        black_box(A[1].0.load(Relaxed));
    }
    println!("{:?}", start.elapsed());
}
*/

fn main() {
    let locked = AtomicBool::new(false);
    let counter = AtomicUsize::new(0);

    std::thread::scope(|s| {
        // Spawn four threads, that each iterate a million times.
        for _ in 0..4 {
            s.spawn(|| {
                for _ in 0..1_000_000 {
                    // Acquire the lock, using the wrong memory ordering.
                    while locked.swap(true, Relaxed) {}
                    compiler_fence(Acquire);

                    // Non-atomically increment the counter, while holding the lock.
                    let old = counter.load(Relaxed);
                    let new = old + 1;
                    counter.store(new, Relaxed);

                    // Release the lock, using the wrong memory ordering.
                    compiler_fence(Release);
                    locked.store(false, Relaxed);
                }
            });
        }
    });

    println!("{}", counter.into_inner());
}
