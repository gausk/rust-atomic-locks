mod condvar;
mod mutex;
mod rwlock;

// fn main() {
//     let m = mutex::Mutex::new(0);
//     std::hint::black_box(&m);
//     let start = std::time::Instant::now();
//     for _ in 0..5_000_000 {
//         *m.lock() += 1;
//     }
//     let duration = start.elapsed();
//     println!("locked {} times in {:?}", *m.lock(), duration);
// }

fn main() {
    let m = mutex::Mutex::new(0);
    std::hint::black_box(&m);
    let start = std::time::Instant::now();
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                for _ in 0..5_000_000 {
                    *m.lock() += 1;
                }
            });
        }
    });
    let duration = start.elapsed();
    println!("locked {} times in {:?}", *m.lock(), duration);
}
